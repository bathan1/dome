CREATE TABLE IF NOT EXISTS todos (
  todo_id INT PRIMARY KEY,
  title TEXT NOT NULL
);

INSERT INTO todos (todo_id, title) VALUES
  (1, 'Do laundry'),
  (2, 'Wash dishes'),
  (3, 'Make bed')
;
