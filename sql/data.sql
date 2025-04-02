insert into stock_index(code, name, exchange, index_code)
values ('000016.SH', '上证50', 'SSE', '000016'),
       ('000300.SH', '沪深300', 'SSE', '000300'),
       ('HSI.HK', '恒生指数', 'HKEX', 'hsi'),
       ('NDX.NS', '纳斯达克100', 'NASDAQ', 'NDX')
;
insert into stock(code, name, exchange, stock_type, stock_code)
values ('000001.SH', '上证指数', 'SSE', 'Index', '000001');
insert into stock(code, name, exchange, stock_type, stock_code)
values ('000016.SH', '上证50', 'SSE', 'Index', '000016');
insert into stock(code, name, exchange, stock_type, stock_code)
values ('000300.SH', '沪深300', 'SSE', 'Index', '000300');
insert into stock(code, name, exchange, stock_type, stock_code)
values ('HSI.HK', '恒生指数', 'HKEX', 'Index', 'HSI');
insert into stock(code, name, exchange, stock_type, stock_code)
values ('NDX.NS', '纳斯达克指数', 'NASDAQ', 'Index', 'NDX');
insert into stock(code, name, exchange, stock_type, stock_code)
values ('SPX.NS', '标普500指数', 'NASDAQ', 'Index', 'SPX');
insert into stock(code, name, exchange, stock_type, stock_code)
values ('QQQ.NS', 'QQQ', 'NASDAQ', 'Fund', 'QQQ');
insert into stock(code, name, exchange, stock_type, stock_code)
values ('PSQ.NS', 'ProShares Short QQQ', 'NASDAQ', 'Fund', 'PSQ');
insert into stock(code, name, exchange, stock_type, stock_code)
values ('SPY.NS', 'SPY', 'NASDAQ', 'Fund', 'SPY');
insert into stock(code, name, exchange, stock_type, stock_code)
values ('7300.HK', '恒生一倍看空', 'HKEX', 'Fund', '7300');
