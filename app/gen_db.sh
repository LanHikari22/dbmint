#!/bin/sh

cd /app/
rm mydb.db
cpp schema.dbml -P -o schema.proc.dbml && \
  dbml_sqlite schema.proc.dbml -f > schema.sqlite && \
  sqlite3 mydb.db < schema.sqlite && \
  sqlite3 mydb.db < data.sqlite 2>&1| grep -v -e 'UNIQUE.*Enum'