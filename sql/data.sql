insert into stock_index(code, name, exchange, index_code)
values ('000016.SH', '上证50', 'SSE', '000016'),
       ('000300.SH', '沪深300', 'SSE', '000300'),
       ('HSI.HK', '恒生指数', 'HKEX', 'hsi'),
       ('NDX.NS', '纳斯达克100', 'NASDAQ', 'NDX')
;
insert into stock_index(code, name, exchange, index_code)
values ('SPX.NS', 'S&P 500', 'NASDAQ', 'SPX')
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

INSERT INTO stock.market_time (id, exchange, start_time, end_time)
VALUES (1, 'SSE', '09:30:00', '11:30:00');
INSERT INTO stock.market_time (id, exchange, start_time, end_time)
VALUES (2, 'SSE', '13:00:00', '15:00:00');
INSERT INTO stock.market_time (id, exchange, start_time, end_time)
VALUES (3, 'SZSE', '09:30:00', '11:30:00');
INSERT INTO stock.market_time (id, exchange, start_time, end_time)
VALUES (4, 'SZSE', '13:00:00', '15:00:00');
INSERT INTO stock.market_time (id, exchange, start_time, end_time)
VALUES (5, 'HKEX', '09:30:00', '12:00:00');
INSERT INTO stock.market_time (id, exchange, start_time, end_time)
VALUES (6, 'HKEX', '13:00:00', '16:00:00');
INSERT INTO stock.market_time (id, exchange, start_time, end_time)
VALUES (7, 'NASDAQ', '09:30:00', '16:00:00');
