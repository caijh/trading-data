use chrono_tz::Tz;
use std::error::Error;
use std::str::FromStr;

/// 股票交易所
/// 枚举中的每个变体都包含一个String类型，用于存放交易所的名称或代码
pub enum Exchange {
    /// 上交所
    SSE,
    /// 深交所
    SZSE,
    /// 港交所
    HKEX,
    /// 纳斯达克交易所
    NASDAQ,
}

impl AsRef<str> for Exchange {
    fn as_ref(&self) -> &str {
        match self {
            Exchange::SSE => "SSE",
            Exchange::SZSE => "SZSE",
            Exchange::HKEX => "HKEX",
            Exchange::NASDAQ => "NASDAQ",
        }
    }
}

impl FromStr for Exchange {
    type Err = Box<dyn Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "SSE" => Ok(Exchange::SSE),
            "SZSE" => Ok(Exchange::SZSE),
            "HKEX" => Ok(Exchange::HKEX),
            "NASDAQ" => Ok(Exchange::NASDAQ),
            _ => Err("Error Exchange code".into()),
        }
    }
}

impl Exchange {
    pub const VALUES: [Self; 4] = [Self::SSE, Self::SZSE, Self::HKEX, Self::NASDAQ];

    pub fn time_zone(&self) -> Tz {
        match self {
            Exchange::SSE => chrono_tz::Asia::Chongqing,
            Exchange::SZSE => chrono_tz::Asia::Chongqing,
            Exchange::HKEX => chrono_tz::Asia::Hong_Kong,
            Exchange::NASDAQ => chrono_tz::America::New_York,
        }
    }

    pub fn int_code(&self) -> usize {
        match self {
            Exchange::SSE => 10,
            Exchange::SZSE => 20,
            Exchange::HKEX => 30,
            Exchange::NASDAQ => 40,
        }
    }

    pub fn stock_code_suffix(&self) -> String {
        match self {
            Exchange::SSE => ".SH".to_string(),
            Exchange::SZSE => ".SZ".to_string(),
            Exchange::HKEX => ".HK".to_string(),
            Exchange::NASDAQ => ".NS".to_string(),
        }
    }
}
