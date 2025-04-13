use chrono_tz::Tz;
use std::error::Error;
use std::str::FromStr;

/// 股票交易所
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

    /// 将字符串转换为交易所枚举类型
    ///
    /// 此函数旨在解析代表不同交易所的字符串代码，并将其转换为相应的枚举值
    /// 它首先将输入字符串转换为大写，然后与预定义的交易所代码进行匹配
    /// 如果匹配成功，则返回对应的交易所枚举值；如果匹配失败，则返回一个错误
    ///
    /// # 参数
    /// * `s`: 一个代表交易所代码的字符串切片
    ///
    /// # 返回值
    /// * `Result<Self, Self::Err>`: 成功时返回对应的交易所枚举值，失败时返回一个自定义错误
    ///
    /// # 示例
    /// let exchange = Exchange::from_str("SSE").unwrap(); 
    /// assert_eq!(exchange, Exchange::SSE); 
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

/// 交易所枚举类型实现
impl Exchange {
    /// 定义所有支持的交易所常量数组
    pub const VALUES: [Self; 4] = [Self::SSE, Self::SZSE, Self::HKEX, Self::NASDAQ];

    /// 返回交易所对应的时间区
    ///
    /// # Returns
    ///
    /// * `Tz` - 交易所的时间区
    pub fn time_zone(&self) -> Tz {
        match self {
            Exchange::SSE => chrono_tz::Asia::Chongqing,
            Exchange::SZSE => chrono_tz::Asia::Chongqing,
            Exchange::HKEX => chrono_tz::Asia::Hong_Kong,
            Exchange::NASDAQ => chrono_tz::America::New_York,
        }
    }

    /// 返回交易所的内部代码
    ///
    /// # Returns
    ///
    /// * `usize` - 交易所的内部代码
    pub fn int_code(&self) -> usize {
        match self {
            Exchange::SSE => 10,
            Exchange::SZSE => 20,
            Exchange::HKEX => 30,
            Exchange::NASDAQ => 40,
        }
    }

    /// 返回交易所的股票代码后缀
    ///
    /// # Returns
    ///
    /// * `String` - 交易所的股票代码后缀
    pub fn stock_code_suffix(&self) -> String {
        match self {
            Exchange::SSE => ".SH".to_string(),
            Exchange::SZSE => ".SZ".to_string(),
            Exchange::HKEX => ".HK".to_string(),
            Exchange::NASDAQ => ".NS".to_string(),
        }
    }
}

