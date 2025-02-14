use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_std::{Addr, Order, StdError, StdResult, Storage, Timestamp},
    cw_storage_plus::{Bound, Index, IndexList, IndexedMap, Item, MultiIndex},
};

use crate::AlarmError;

type TimeSeconds = u64;
pub type AlarmsCount = u32;
pub type Id = u64;

fn as_seconds(from: Timestamp) -> TimeSeconds {
    from.seconds()
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct Alarm {
    pub time: TimeSeconds,
    pub addr: Addr,
}

struct AlarmIndexes<'a> {
    alarms: MultiIndex<'a, TimeSeconds, Alarm, Id>,
}

impl<'a> IndexList<Alarm> for AlarmIndexes<'a> {
    fn get_indexes(&self) -> Box<dyn Iterator<Item = &'_ dyn Index<Alarm>> + '_> {
        let v: Vec<&dyn Index<Alarm>> = vec![&self.alarms];

        Box::new(v.into_iter())
    }
}

pub struct Alarms<'a> {
    namespace_alarms: &'a str,
    namespace_index: &'a str,
    next_id: Item<'a, Id>,
}

impl<'a> Alarms<'a> {
    pub const fn new(
        namespace_alarms: &'a str,
        namespace_index: &'a str,
        namespace_next_id: &'a str,
    ) -> Self {
        Self {
            namespace_alarms,
            namespace_index,
            next_id: Item::new(namespace_next_id),
        }
    }

    fn alarms(&self) -> IndexedMap<'a, TimeSeconds, Alarm, AlarmIndexes<'a>> {
        let indexes = AlarmIndexes {
            alarms: MultiIndex::new(|_, d| d.time, self.namespace_alarms, self.namespace_index),
        };

        IndexedMap::new(self.namespace_alarms, indexes)
    }

    pub fn add(&self, storage: &mut dyn Storage, addr: Addr, time: Timestamp) -> StdResult<Id> {
        let id = self.next_id.may_load(storage)?.unwrap_or_default();

        let alarm = Alarm {
            time: as_seconds(time),
            addr,
        };

        self.alarms().save(storage, id, &alarm)?;

        self.next_id.save(storage, &id.wrapping_add(1))?;

        Ok(id)
    }

    pub fn remove(&self, storage: &mut dyn Storage, id: Id) -> StdResult<()> {
        self.alarms().remove(storage, id)
    }

    fn alarms_selection<'b>(
        &self,
        storage: &'b dyn Storage,
        ctime: Timestamp,
        max_id: Id,
    ) -> impl Iterator<Item = Result<(Id, Alarm), StdError>> + 'b {
        self.alarms().idx.alarms.range(
            storage,
            None,
            Some(Bound::inclusive((as_seconds(ctime), max_id))),
            Order::Ascending,
        )
    }

    pub fn notify<D>(
        &self,
        storage: &mut dyn Storage,
        dispatcher: D,
        ctime: Timestamp,
        max_count: AlarmsCount,
    ) -> Result<D, AlarmError>
    where
        D: AlarmDispatcher,
    {
        let max_id = self.next_id.may_load(storage)?.unwrap_or_default();

        self.alarms_selection(storage, ctime, max_id)
            .take(max_count.try_into()?)
            .try_fold(dispatcher, |dispatcher, alarm| {
                let (id, alarm) = alarm?;

                dispatcher.send_to(id, alarm.addr)
            })
    }

    pub fn any_alarm(&self, storage: &dyn Storage, ctime: Timestamp) -> Result<bool, AlarmError> {
        let max_id = self.next_id.may_load(storage)?.unwrap_or_default();

        Ok(self.alarms_selection(storage, ctime, max_id).any(|_| true))
    }
}

pub trait AlarmDispatcher
where
    Self: Sized,
{
    fn send_to(self, id: Id, addr: Addr) -> Result<Self, AlarmError>;
}

#[cfg(test)]
pub mod tests {
    use sdk::cosmwasm_std::testing;

    use super::*;

    #[derive(Default)]
    struct MockAlarmDispatcher(pub Vec<Id>);

    impl AlarmDispatcher for MockAlarmDispatcher {
        fn send_to(mut self, id: Id, _addr: Addr) -> Result<Self, AlarmError> {
            self.0.push(id);

            Ok(self)
        }
    }

    impl MockAlarmDispatcher {
        fn clean_alarms(&self, storage: &mut dyn Storage, alarms: &Alarms<'_>) -> StdResult<()> {
            self.0.iter().try_for_each(|&id| alarms.remove(storage, id))
        }
    }

    #[test]
    fn test_add() {
        let alarms = Alarms::new("alarms", "alarms_idx", "alarms_next_id");
        let storage = &mut testing::mock_dependencies().storage;

        let t0 = Timestamp::from_seconds(0);
        let t1 = Timestamp::from_seconds(1);
        let t2 = Timestamp::from_seconds(2);
        let t3 = Timestamp::from_seconds(3);
        let addr1 = Addr::unchecked("addr1");
        let addr2 = Addr::unchecked("addr2");
        let addr3 = Addr::unchecked("addr3");

        assert!(!alarms.any_alarm(storage, t3).unwrap());

        assert_eq!(alarms.add(storage, addr1, t1), Ok(0));
        // same timestamp
        assert_eq!(alarms.add(storage, addr2, t1), Ok(1));
        // different timestamp
        assert_eq!(alarms.add(storage, addr3, t2), Ok(2));

        assert!(!alarms.any_alarm(storage, t0).unwrap());
        assert!(alarms.any_alarm(storage, t3).unwrap());
    }

    #[test]
    fn test_remove() {
        let alarms = Alarms::new("alarms", "alarms_idx", "alarms_next_id");
        let storage = &mut testing::mock_dependencies().storage;
        let dispatcher = MockAlarmDispatcher::default();
        let t1 = Timestamp::from_seconds(10);
        let t2 = Timestamp::from_seconds(20);
        let addr1 = Addr::unchecked("addr1");
        let addr2 = Addr::unchecked("addr2");
        let addr3 = Addr::unchecked("addr3");
        let err_id = 4;

        // same time stamp
        let id1 = alarms.add(storage, addr1, t1).expect("can't set alarms");
        let id2 = alarms.add(storage, addr2, t1).expect("can't set alarms");
        // different timestamp
        let id3 = alarms.add(storage, addr3, t2).expect("can't set alarms");

        assert_eq!(alarms.remove(storage, id1), Ok(()));
        assert_eq!(alarms.remove(storage, id3), Ok(()));

        // unknown recipient: cw_storage_plus Map does't throw an Err, when removes unknown item.
        alarms
            .remove(storage, err_id)
            .expect("remove alarm with unknown id");

        let dispatcher = alarms.notify(storage, dispatcher, t2, 100).unwrap();
        assert_eq!(dispatcher.0, [id2]);
    }

    #[test]
    fn test_notify() {
        let alarms = Alarms::new("alarms", "alarms_idx", "alarms_next_id");
        let storage = &mut testing::mock_dependencies().storage;
        let dispatcher = MockAlarmDispatcher::default();
        let t1 = Timestamp::from_seconds(1);
        let t2 = Timestamp::from_seconds(2);
        let t3 = Timestamp::from_seconds(3);
        let t4 = Timestamp::from_seconds(4);
        let addr1 = Addr::unchecked("addr1");
        let addr2 = Addr::unchecked("addr2");
        let addr3 = Addr::unchecked("addr3");
        let addr4 = Addr::unchecked("addr4");

        // same timestamp
        let id1 = alarms.add(storage, addr1, t1).expect("can't set alarms");
        let id2 = alarms.add(storage, addr2, t1).expect("can't set alarms");
        // different timestamp
        let id3 = alarms.add(storage, addr3, t2).expect("can't set alarms");
        // rest
        alarms.add(storage, addr4, t4).expect("can't set alarms");

        let dispatcher = alarms.notify(storage, dispatcher, t1, 100).unwrap();
        assert_eq!(dispatcher.0, [id1, id2]);
        dispatcher
            .clean_alarms(storage, &alarms)
            .expect("can't clean up alarms db");

        let dispatcher = MockAlarmDispatcher::default();
        let dispatcher = alarms.notify(storage, dispatcher, t3, 100).unwrap();
        assert_eq!(dispatcher.0, [id3]);
    }

    #[test]
    fn test_id_overflow() {
        let mut deps = testing::mock_dependencies();
        let alarms = Alarms::new("alarms", "alarms_idx", "alarms_next_id");

        let id_item: Item<'_, Id> = Item::new("alarms_next_id");
        id_item.save(&mut deps.storage, &(Id::MAX)).unwrap();

        let id = alarms
            .add(
                &mut deps.storage,
                Addr::unchecked("test"),
                Timestamp::from_seconds(1),
            )
            .unwrap();
        assert_eq!(id, Id::MAX);

        // overflow
        let id = alarms
            .add(
                &mut deps.storage,
                Addr::unchecked("test"),
                Timestamp::from_seconds(2),
            )
            .unwrap();
        assert_eq!(id, 0);
    }
}
