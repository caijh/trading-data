use chrono_tz::Tz;
use std::fmt;
use std::str::FromStr;

/// 股票交易所
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl Exchange {
    /// 定义所有支持的交易所常量数组
    pub const VALUES: [Self; 4] = [Self::SSE, Self::SZSE, Self::HKEX, Self::NASDAQ];

    /// 返回交易所对应的时间区
    pub fn time_zone(&self) -> Tz {
        match self {
            Exchange::SSE | Exchange::SZSE => chrono_tz::Asia::Chongqing,
            Exchange::HKEX => chrono_tz::Asia::Hong_Kong,
            Exchange::NASDAQ => chrono_tz::America::New_York,
        }
    }

    /// 返回交易所的内部代码
    pub fn int_code(&self) -> usize {
        match self {
            Exchange::SSE => 10,
            Exchange::SZSE => 20,
            Exchange::HKEX => 30,
            Exchange::NASDAQ => 40,
        }
    }

    /// 返回交易所的股票代码后缀
    pub fn stock_code_suffix(&self) -> &'static str {
        match self {
            Exchange::SSE => ".SH",
            Exchange::SZSE => ".SZ",
            Exchange::HKEX => ".HK",
            Exchange::NASDAQ => ".NS",
        }
    }
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

impl fmt::Display for Exchange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl FromStr for Exchange {
    type Err = ExchangeError;

    /// 将字符串转换为交易所枚举类型
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "SSE" => Ok(Exchange::SSE),
            "SZSE" => Ok(Exchange::SZSE),
            "HKEX" => Ok(Exchange::HKEX),
            "NASDAQ" => Ok(Exchange::NASDAQ),
            _ => Err(ExchangeError::InvalidCode(s.to_string())),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ExchangeError {
    InvalidCode(String),
}

impl fmt::Display for ExchangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExchangeError::InvalidCode(code) => write!(f, "Invalid exchange code: {}", code),
        }
    }
}

impl std::error::Error for ExchangeError {}
