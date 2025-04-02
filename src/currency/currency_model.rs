use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};

/**
 * 表示两种货币之间的汇率信息。
 *
 * # 属性
 * - `currency_from`: 起始货币的标识。
 * - `currency_to`: 目标货币的标识。
 * - `buy_price`: 购汇
 * - `sell_price`: 结汇
 */
#[derive(Serialize, Deserialize, Clone)]
pub struct CurrencyRate {
    pub from: String,
    pub to: String,
    pub buy_price: BigDecimal,
    pub sell_price: BigDecimal,
}
