from typing import List, Tuple
import dbml_sqlite
from pydbml import Database
from dbml_sqlite import PyDBML
from pydbml.classes import Table, Column, Note, Reference

from .common import MOUNT_PATH

import unittest


class DbmlUnitTests(unittest.TestCase):
    def test_something(self) -> None:
        self.assertTrue(False, "rigorous test :)")

    def get_ex000_data(self) -> Tuple[List[Table], List[Reference]]:
        table0 = (
            Table(
                "Users",
                columns=[
                    Column("id", "integer", pk=True),
                    Column("username", "varchar"),
                    Column("role", "varchar"),
                    Column("created_at", "varchar"),
                ],
            )
        )

        table1 = (
            Table(
                "Posts",
                columns=[
                    Column("id", "integer", pk=True),
                    Column("title", "varchar"),
                    Column("body", "varchar", note=Note("Content of the post")),
                    Column("status", "varchar"),
                    Column("created_at", "varchar"),
                ],
            )
        )

        table2 = (
            Table(
                "User_Owns_Posts",
                columns=[
                    Column("id", "integer", pk=True),
                    Column("from_user_id", "integer"),
                    Column("to_post_id", "integer"),
                ],
                comment="A one-to-many relationship between users and posts",
            )
        )

        expected_tables = [table0, table1, table2]

        expected_refs = [
            Reference('-', table2.columns[1], table0.columns[0]),
            Reference('-', table2.columns[2], table1.columns[0]),
        ]

        return (expected_tables, expected_refs)



    def test_example_parsing_dbml(self) -> None:
        dbml_path = f"{MOUNT_PATH}ex000.dbml"

        self.assertTrue(
            dbml_sqlite.validDBMLFile(dbml_path), f"Invalid dbml file: {dbml_path}"
        )

        dbml_data = PyDBML.parse_file(dbml_path)

        # prints [Table('Users', [Column('id', 'integer', pk=True), Column('username', 'varchar'), Column('role', 'varchar'), Column('created_at', 'varchar')]), Table('Posts', [Column('id', 'integer', pk=True), Column('title', 'varchar'), Column('body', 'varchar', note=Note('Content of the post')), Column('status', 'varchar'), Column('created_at', 'varchar')]), Table('User_Owns_Posts', [Column('id', 'integer', pk=True), Column('from_user_id', 'integer'), Column('to_post_id', 'integer')])]
        # print(dbml_data.tables)

        expected_tables, expected_refs = self.get_ex000_data()

        self.assertEqual(dbml_data.tables, expected_tables)
        self.assertEqual(dbml_data.refs, expected_refs)

        print(dbml_data.__dict__)

        self.assertTrue(True, "Toggle to show example")

    def test_example_writing_dbml(self) -> None:
        dbml_path = f"{MOUNT_PATH}ex000.dbml"

        tables, refs = self.get_ex000_data()

        db = Database()

        for table in tables:
            db.add_table(table)

        for ref in refs:
            db.add_reference(ref)

        with open(dbml_path, 'r') as f:
            content = f.read()

        with open(f'{MOUNT_PATH}b.dbml', 'w') as f:
            f.write(db.dbml)

        self.assertEqual(content.strip(), db.dbml)

        self.assertTrue(True, "Toggle to show example")
