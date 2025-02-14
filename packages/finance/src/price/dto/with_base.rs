use crate::{
    coin::{Coin, CoinDTO},
    currency::{visit_any_on_ticker, AnyVisitor, AnyVisitorResult, Currency, Group},
    error::Error,
    price::{self},
};

use super::{PriceDTO, WithBase};

struct QuoteCVisitor<'a, QuoteG, C, Cmd>
where
    C: Currency,
{
    base: Coin<C>,
    quote_dto: &'a CoinDTO<QuoteG>,
    cmd: Cmd,
}

impl<'a, QuoteG, C, Cmd> AnyVisitor for QuoteCVisitor<'a, QuoteG, C, Cmd>
where
    C: Currency,
    Cmd: WithBase<C>,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    #[track_caller]
    fn on<QuoteC>(self) -> AnyVisitorResult<Self>
    where
        QuoteC: Currency,
    {
        let amount_quote =
            Coin::<QuoteC>::try_from(self.quote_dto).expect("Got different currency in visitor!");
        let price = price::total_of(self.base).is(amount_quote);
        self.cmd.exec(price)
    }
}

#[track_caller]
pub fn execute<G, QuoteG, Cmd, C>(
    price: &PriceDTO<G, QuoteG>,
    cmd: Cmd,
) -> Result<Cmd::Output, Cmd::Error>
where
    G: Group,
    QuoteG: Group,
    Cmd: WithBase<C>,
    C: Currency,
    Error: Into<Cmd::Error>,
{
    visit_any_on_ticker::<QuoteG, _>(
        &price.amount_quote.ticker().clone(),
        QuoteCVisitor {
            base: Coin::<C>::try_from(&price.amount).expect("Got different currency in visitor!"),
            quote_dto: &price.amount_quote,
            cmd,
        },
    )
}
