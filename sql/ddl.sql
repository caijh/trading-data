create table stock.fund
(
    code     varchar(6)  not null
        primary key,
    name     varchar(20) null comment '基金名称',
    exchange varchar(10) null comment '交易所'
)
    comment '基金';

create table stock.index_constituent
(
    index_code varchar(10) not null comment '指数代码',
    stock_code varchar(10) not null comment '股票代码',
    stock_name varchar(10) null,
    primary key (index_code, stock_code)
)
    comment '指数成分股';

create table stock.market_holiday
(
    id    bigint unsigned not null
        primary key,
    year  int unsigned    null,
    month int unsigned    null,
    day   int unsigned    null
)
    comment '市场休假日';

create table stock.stock
(
    code       varchar(10)                 not null
        primary key,
    name       varchar(20)                 not null,
    exchange   varchar(10)                 null,
    stock_type varchar(10) default 'Stock' not null comment '股票类型：Stock/Index',
    to_code    varchar(10)                 null comment '将code转其他code'
)
    comment '股市列表';

create table stock.stock_daily_price
(
    code   varchar(10)     not null,
    date   bigint unsigned not null,
    open   decimal(10, 3)  null,
    close  decimal(10, 3)  null,
    high   decimal(10, 3)  null,
    low    decimal(10, 3)  null,
    volume decimal(18, 2)  null,
    amount decimal(18, 2)  null,
    zf     decimal(10, 2)  null,
    hs     decimal(10, 2)  null,
    zd     decimal(10, 2)  null,
    zde    decimal(10, 2)  null,
    primary key (code, date)
)
    comment '股票每日行情数据';

create table stock.stock_daily_price_sync_record
(
    code    varchar(10)          not null
        primary key,
    date    bigint unsigned      not null,
    updated tinyint(1) default 0 null
)
    comment '股票每日股价同步记录';

create table stock.stock_index
(
    code     varchar(10) not null comment '股指代码'
        primary key,
    name     varchar(10) null comment '股指名称',
    exchange varchar(2)  null comment '所属交易所'
)
    comment '股票指数';

create table market_time
(
    id         bigint unsigned auto_increment
        primary key,
    exchange   varchar(10) not null comment '交易所',
    start_time time        not null comment '开始时间',
    end_time   time        not null comment '结束时间'
)
    comment '市场交易时间';

