use cosmwasm_std::Timestamp;
use serde::Serialize;

use finance::{coin::Coin, currency::Currency};
use lpp::stub::lender::LppLender as LppLenderTrait;
use market_price_oracle::stub::Oracle as OracleTrait;
use platform::batch::Batch;
use profit::stub::Profit as ProfitTrait;
use time_alarms::stub::TimeAlarms as TimeAlarmsTrait;

use crate::{
    error::ContractResult,
    lease::{Lease, LeaseDTO},
    loan::RepayReceipt,
};

use super::IntoDTOResult;

impl<'r, Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>
    Lease<'r, Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>
where
    Lpn: Currency + Serialize,
    Lpp: LppLenderTrait<Lpn>,
    TimeAlarms: TimeAlarmsTrait,
    Oracle: OracleTrait<Lpn>,
    Profit: ProfitTrait,
    Asset: Currency + Serialize,
{
    pub(crate) fn repay(
        mut self,
        lease_amount: Coin<Asset>,
        payment: Coin<Lpn>,
        now: Timestamp,
    ) -> ContractResult<Result<Lpn>> {
        let receipt = self.no_reschedule_repay(payment, now)?;

        self.reschedule_on_repay(lease_amount, &now)?;

        let IntoDTOResult {
            dto: lease_dto,
            batch,
        } = self.into_dto();

        Ok(Result {
            batch,
            lease_dto,
            receipt,
        })
    }

    pub(super) fn no_reschedule_repay(
        &mut self,
        payment: Coin<Lpn>,
        now: Timestamp,
    ) -> ContractResult<RepayReceipt<Lpn>> {
        self.loan
            .repay(payment, now, self.lease_addr.clone())
            .map_err(Into::into)
    }
}

pub(crate) struct Result<Lpn>
where
    Lpn: Currency,
{
    pub batch: Batch,
    pub lease_dto: LeaseDTO,
    pub receipt: RepayReceipt<Lpn>,
}
