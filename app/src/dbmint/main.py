from dataclasses import dataclass
import sys
import os
import argparse
import datetime
import enum
import checkpipe as pipe
from result import Result, Ok, Err
from typing import Dict, Generic, Iterable, Optional, Self, Tuple, List, TypeVar, TypedDict, Union
import unittest

from .common import *

# For testing, set this to './tmp/'. For production, '/mnt/'
# MOUNT_PATH = './tmp/'
MOUNT_PATH = '/mnt/'

T = TypeVar('T')

Error = AppError[Union[
    'SqliteDumpParser.Errno',
    'TableCsvDir.Errno',
    'SyncCommandsImpl.RenameParseErrno'
]]


class SqliteDumpParser:
    """
    Able to parse minimum table data and row inserts from "sqlite3 <db_file> ".dump"
    """

    class Errno(enum.Enum):
        TableParseError = enum.auto()
        InsertParseError = enum.auto()
        InternalError = enum.auto()

    class TableColumnsData:
        def __init__(
            self, name: str, 
            varnames: List[str], 
            vartypes: List[str], 
            varsettings: List[str], 
            foreign_key_defns: List[Tuple[str, Tuple[str, str]]]):

            self.name = name
            self.varnames = varnames
            self.vartypes = vartypes
            self.varsettings = varsettings
            self.foreign_key_defns = foreign_key_defns

        def __str__(self) -> str:
            return str(self.__dict__)

    class InsertData:
        def __init__(self, table_name: str, values: List[str]):
            self.table_name = table_name
            self.values = values
        
        def __str__(self) -> str:
            return str(self.__dict__)

    @staticmethod
    def parse_tables_and_columns_from_dump(dump: str) -> Result[List[TableColumnsData], Error]:
        parsing_table = False

        lines = dump.splitlines()

        mut_results = []

        # Temporary storage for constructing a TableColumnsData
        mut_table_name = ''
        mut_varnames: List[str] = []
        mut_vartypes: List[str] = []
        mut_varsettings: List[str] = []
        mut_foreign_key_defns: List[Tuple[str, Tuple[str, str]]] = []

        for line in lines:
            line = line.strip()

            if not parsing_table:
                # Filter the lines until we get to "CREATE TABLE("
                table_syntax_start = 'CREATE TABLE '
                table_syntax_end = '('
                if line.startswith(table_syntax_start) and line.endswith(table_syntax_end):
                    line = line[len(table_syntax_start):-len(table_syntax_end)]
                    mut_table_name = line.strip()
                    parsing_table = True
                else:
                    continue
            else:
                if line == ');':
                    parsing_table = False

                    if (len(mut_varnames) != 0 and 
                        len(mut_vartypes) != 0 and 
                        len(mut_varsettings) != 0 and 
                        mut_table_name != ''):

                        if len(mut_varnames) != len(mut_vartypes):
                            return Err(Error(
                                SqliteDumpParser.Errno.TableParseError, 
                                f'varnames and vartypes of different len: {len(mut_varnames)} != {len(mut_vartypes)} for line {line}'))

                        if len(mut_varnames) != len(mut_varsettings):
                            return Err(Error(
                                SqliteDumpParser.Errno.TableParseError, 
                                f'varnames and varsettings of different len: {len(mut_varnames)} != {len(mut_varsettings)} for line {line}'))

                        mut_results.append(SqliteDumpParser.TableColumnsData(
                            mut_table_name, mut_varnames, mut_vartypes, mut_varsettings, mut_foreign_key_defns))

                        # Clear temporary storage for next table
                        mut_table_name = ''
                        mut_varnames = []
                        mut_vartypes = []
                        mut_varsettings = []
                        mut_foreign_key_defns = []
                    else:
                        return Err(Error(
                            SqliteDumpParser.Errno.InternalError, 
                            f'Failed to parse varnames/vartypes/varsettings/table_name'))
                else:
                    words = list(line.split(' '))
                    if len(words) < 2:
                        return Err(Error(
                            SqliteDumpParser.Errno.TableParseError, 
                            f'need at least varname vartype in line {line}'))
                    else:
                        # We are not interested in key link lines and other non-member specifying lines
                        if words[0] in 'FOREIGN':
                            if len(words) != 4:
                                return Err(Error(
                                    SqliteDumpParser.Errno.TableParseError, 
                                    f'foreign key declaration requires 4 words: {line}'))

                            if 'KEY(' not in words[1]:
                                return Err(Error(
                                    SqliteDumpParser.Errno.TableParseError, 
                                    f'foreign key declaration are malformed: {line}'))
                            
                            from_key = words[1].replace('KEY(', '').replace(')', '').strip()
                            to_key = words[3].replace(',', '').strip()
                            to_key_table = to_key.split('(')[0]
                            to_key_col = to_key.split('(')[1].replace(')', '')

                            mut_foreign_key_defns.append((from_key, (to_key_table, to_key_col)))

                            continue
                        else:
                            mut_varnames.append(words[0])
                            mut_vartypes.append(words[1])
                            if len(words) > 2:
                                mut_varsettings.append(' '.join(words[2:]))
                            else:
                                mut_varsettings.append('')
        
        return Ok(mut_results)

    @staticmethod
    def parse_inserts(s: str) -> Result[List[InsertData], Error]:
        lines = s.splitlines()

        result = []

        for line in lines:
            prefix = 'INSERT INTO '
            if line.startswith(prefix):
                values_syntax_begin = 'VALUES('
                values_syntax_end = ');'
                if values_syntax_begin not in line or not line.endswith(values_syntax_end):
                    return Err(Error(
                        SqliteDumpParser.Errno.InsertParseError, 
                        f'Expected VALUES(...) in line {line}'))

                line = line[len(prefix):]
                words = line.split(' ')
                table_name = words[0]

                values = (
                    line[line.find('(')+1:line.find(')')]
                        | pipe.Of[str]
                        .to(lambda s: list(s.split(',')))
                        | pipe.OfIter[str]
                        .map(lambda s: s.strip())
                        | pipe.OfIter[str]
                        .to_list()
                )

                result.append(SqliteDumpParser.InsertData(table_name, values))

        return Ok(result)

    @staticmethod
    def create_insert_dump(inserts: List['SqliteDumpParser.InsertData']) -> str:
        sql_script = ''

        for insert in inserts:
            values_str = ','.join(
                insert.values 
                    | pipe.OfIter[str]
                    .map(lambda s: s.strip())
                    | pipe.OfIter[str]
                    .to_list()
            )
            sql_script += f'INSERT INTO {insert.table_name} VALUES({values_str});\n'
        
        return sql_script


class TableCsvDir:
    class Errno(enum.Enum):
        TableWithNoColumns = enum.auto()
        ValuesOfDiffLenToCols = enum.auto()
        InvalidCsvTable = enum.auto()
        pass

    @staticmethod
    def export_csv(dir_name: str, table_columns: SqliteDumpParser.TableColumnsData, 
                    inserts: List[SqliteDumpParser.InsertData]) -> IO[Result[None, Error]]:

        table_name = table_columns.name

        with open(f'{MOUNT_PATH}{dir_name}/{table_name}.csv', 'w') as fw:
            # Let's write the columns line first
            column_line = ''
            for column in table_columns.varnames:
                column_line += f'{column},'
            
            if not column_line.endswith(','):
                return IO(Err(Error(
                    TableCsvDir.Errno.TableWithNoColumns, 
                    data={'table_columns': table_columns})))
            
            column_line = column_line[:-len(',')]

            fw.write(column_line + '\n')

            inserts = (
                inserts
                    | pipe.OfIter[SqliteDumpParser.InsertData]
                    .filter(lambda ins: ins.table_name == table_name)
                    | pipe.OfIter[SqliteDumpParser.InsertData]
                    .to_list()
            )

            for insert in inserts:
                insert_line = ''

                if len(insert.values) != len(table_columns.varnames):
                    return IO(Err(Error(
                        TableCsvDir.Errno.ValuesOfDiffLenToCols, 
                        data={'table_columns': table_columns,
                              'insert': insert})))

                for value in insert.values:
                    insert_line += f'{value},'

                # Has to be non-empty since columns have to be non-empty and 
                # it must be same count as columns
                insert_line = insert_line[:-len(',')]

                fw.write(insert_line + '\n')
        
        return IO(Ok(None))

    @staticmethod
    def export_directory(dir_name: str, tables_columns: List[SqliteDumpParser.TableColumnsData], 
                         inserts: List[SqliteDumpParser.InsertData]) -> IO[None]:
        os.system(f'mkdir -p {MOUNT_PATH}{dir_name}')

        for table_columns in tables_columns:
            res = (
                TableCsvDir.export_csv(dir_name, table_columns, inserts)
                    .extract(caller_returns_IO=True)
            )

            if res.is_err():
                print(f'Error: {res.unwrap_err()}')
        
        return IO(None)

    @staticmethod
    def import_csv(csv_path: str) -> IO[List[SqliteDumpParser.InsertData]]:
        with open(csv_path, 'r') as fr:
            lines = fr.readlines()
        
        csv_path_nodir = os.path.split(csv_path)[1]
        csv_path_no_ext = os.path.splitext(csv_path_nodir)[0]
        table_name = csv_path_no_ext

        result = []
        skipped_column_line = False

        for line in lines:
            if not skipped_column_line:
                skipped_column_line = True
                continue

            values = (
                list(line.split(','))
                    | pipe.OfIter[str]
                    .map(lambda s: s.strip())
                    | pipe.OfIter[str]
                    .to_list()
            )
            
            result.append(SqliteDumpParser.InsertData(table_name, values))
        
        return IO(result)

    @staticmethod
    def import_directory(dir_name: str) -> IO[List[SqliteDumpParser.InsertData]]:
        csv_filenames = (
            os.listdir(f'{MOUNT_PATH}{dir_name}')
                | pipe.OfIter[str]
                .filter(lambda s: s.endswith('.csv'))
                | pipe.OfIter[str]
                .to_list()
        )

        results = []

        for csv_filename in csv_filenames: (
            TableCsvDir.import_csv(f'{MOUNT_PATH}{dir_name}/{csv_filename}')
                .extract(caller_returns_IO=True)
                | pipe.Of[List[SqliteDumpParser.InsertData]]
                .to(lambda xs: results.extend(xs))
        )

        
        return IO(results)

E = TypeVar('E')

class SyncCommandsImpl:
    class RenameEntities(enum.Enum):
        RENAME_COLUMN = enum.auto()
        RENAME_TABLE = enum.auto()

    class RenameParseErrno(enum.Enum):
        InternalParseError = enum.auto()
        InvalidSchemaError = enum.auto()

    @dataclass
    class Rename():
        entity_type: 'SyncCommandsImpl.RenameEntities'
        table: str
        
        before: str
        """
        Includes type and settings. The line marked for rename.
        """

        after: str
        """
        Includes type and settings. The line after the line marked for rename. (to replace with)
        """

        orig_s: str

        @classmethod
        def from_str(cls, dbml_schema_content: str) -> Result[List['SyncCommandsImpl.Rename'], Error]:
            """
            We need to parse the dbml schema for rename commands. Rename commands are lines which start with `-`, 
            followed by a line with the new rename.
            """

            dbml_schema_lines = dbml_schema_content.splitlines()

            rename_prefix: str = '-'
            table_defn_prefix: str = rename_prefix + ' Table' 


            # We want to keep the `-` lines and immediately the next line
            sync_rename_linenos = (
                dbml_schema_lines
                    | pipe.OfIter[str]
                    .enumerate()
                    | pipe.OfIter[Tuple[int, str]]
                    .filter(pipe.tup2_unpack(lambda _, line:
                        line.strip().startswith(rename_prefix)
                    ))
                    | pipe.OfIter[Tuple[int, str]]
                    .map(pipe.tup2_unpack(lambda i, _: i))
                    | pipe.OfIter[int]
                    .to_list()
            )

            # We want not just the line but the one next to it, to be processd sequentially
            sync_rename_linenos_lines = (
                dbml_schema_lines
                    | pipe.OfIter[str]
                    .enumerate()
                    | pipe.OfIter[Tuple[int, str]]
                    .filter(pipe.tup2_unpack(lambda i, _:
                        i in sync_rename_linenos or (i-1) in sync_rename_linenos
                    ))
                    | pipe.OfIter[Tuple[int, str]]
                    .to_list()
            )

            # For all table definitions, we want to grab them as is, but if they are exactly below a 
            # commented out definition to be renamed, we grab that one instead
            orig_table_defn_lines = (
                dbml_schema_lines
                    | pipe.OfIter[str]
                    .map(lambda line: line.strip())
                    | pipe.OfIter[str]
                    .enumerate()
                    | pipe.OfIter[Tuple[int, str]]
                    .filter(pipe.tup2_unpack(lambda _, line:
                        line.startswith('Table ') or
                        line.startswith(table_defn_prefix)
                    ))
                    | pipe.OfIter[Tuple[int, str]]
                    .map(pipe.tup2_unpack(lambda i, line:
                        # Check the previous line. If it's a rename command, grab it instead
                        (i-1, dbml_schema_lines[i-1])
                            if i > 0 and dbml_schema_lines[i-1].strip().startswith(table_defn_prefix) else
                        (i, line)
                    ))
                    | pipe.OfIter[Tuple[int, str]]
                    .to_list()
            )

            def get_corresponding_table_defn(lineno: int, line: str) -> Result[str, str]:
                table_defns_prior_or_equal_to_line = (
                    orig_table_defn_lines
                        | pipe.OfIter[Tuple[int, str]]
                        .filter(pipe.tup2_unpack(lambda i, _: lineno >= i))
                        | pipe.OfIter[Tuple[int, str]]
                        .to_list()
                )

                if len(table_defns_prior_or_equal_to_line) == 0:
                    return Err(f'Could not resolve a table for {lineno} {line}')
                
                defn = (
                    table_defns_prior_or_equal_to_line[-1][1]
                        | pipe.Of[str]
                        .to(lambda defn_line:
                            defn_line
                                .replace(table_defn_prefix, '')
                                .replace('Table', '')
                                .replace('{', '')
                                .strip()
                        )
                )

                return (
                    Ok(defn)
                )
                

            # We need to know for each selected line, its corresponding prior table definition
            enumerated_lines_with_defn_res = (
                sync_rename_linenos_lines
                    | pipe.OfIter[Tuple[int, str]]
                    .map(pipe.tup2_unpack(lambda i, line:
                        get_corresponding_table_defn(i, line)
                            .and_then(lambda defn:
                                Ok((i, line, defn))
                            )
                    ))
                    | pipe.OfResultIter[Tuple[int, str, str], str]
                    .flatten()
            )

            if enumerated_lines_with_defn_res.is_err():
                return Err(Error(
                    SyncCommandsImpl.RenameParseErrno.InvalidSchemaError,
                    enumerated_lines_with_defn_res.unwrap_err()
                ))

            enumerated_lines_with_defn = enumerated_lines_with_defn_res.unwrap()

            def parse_before_line(line: str, table_ident: str) -> Result[Tuple[bool, str, str, str], str]:
                if not line.strip().startswith(rename_prefix):
                    return Err(f"Invalid before line. Must have a sync rename command: {line}")
                
                is_table = 'Table' in line
                
                ident = (
                    line
                        .replace(rename_prefix, '')
                        .replace(table_defn_prefix, '')
                        .replace('Table', '')
                        .replace('{', '')
                        .strip()
                )

                if is_table and ident != table_ident:
                    return Err(f"Table ident {ident} and {table_ident} must be identical. Line: {line}")


                return Ok((is_table, ident, table_ident, line))

            def parse_after_line(line: str, table_ident: str) -> Result[Tuple[bool, str, str, str], str]:
                if line.strip().startswith(rename_prefix):
                    return Err(f"Invalid after line. Must not have a sync rename command. Line: \"{line}\"")
                
                is_table = 'Table' in line
                
                ident = (
                    line
                        .replace('Table', '')
                        .replace('{', '')
                        .strip()
                )

                return Ok((is_table, ident, table_ident, line))

            parsed_items_res = (
                enumerated_lines_with_defn
                    | pipe.OfIter[Tuple[int, str, str]]
                    .enumerate()
                    | pipe.OfIter[Tuple[int, Tuple[int, str, str]]]
                    .map(pipe.tup2_right_tup3_flatten)
                    | pipe.OfIter[Tuple[int, int, str, str]]
                    .map(pipe.tup4_unpack(lambda i, _lineno, line, defn:
                        parse_before_line(line, defn)
                            if i % 2 == 0 else
                        parse_after_line(line, defn)
                    ))
                    | pipe.OfResultIter[Tuple[bool, str, str, str], str]
                    .flatten()
            )

            if parsed_items_res.is_err():
                return Err(Error(
                    SyncCommandsImpl.RenameParseErrno.InternalParseError,
                    parsed_items_res.unwrap_err()
                ))
            
            parsed_items = parsed_items_res.unwrap()

            # We need to group the items by two
            if len(parsed_items) % 2 != 0:
                return Err(Error(
                    SyncCommandsImpl.RenameParseErrno.InternalParseError,
                    "Expected parsed items to be divisible by 2"
                ))
            
            parsed_items_grouped2 = (
                range(0, len(parsed_items) // 2)
                    | pipe.OfIter[int]
                    .map(lambda i:
                        (parsed_items[i], parsed_items[i+1])
                    )
                    | pipe.OfIter[Tuple[Tuple[bool, str, str, str], Tuple[bool, str, str, str]]]
                    .to_list()
            )

            def construct_self(tup_before: Tuple[bool, str, str, str], tup_after: Tuple[bool, str, str, str]) -> SyncCommandsImpl.Rename:
                is_table1, ident1, table_ident1, line1 = tup_before
                is_table2, ident2, table_ident2, line2 = tup_after

                return (
                    SyncCommandsImpl.Rename(
                        SyncCommandsImpl.RenameEntities.RENAME_TABLE 
                            if is_table1 else 
                        SyncCommandsImpl.RenameEntities.RENAME_COLUMN,
                        table_ident1,
                        ident1,
                        ident2,
                        line1
                    )
                )

            results = (
                parsed_items_grouped2
                    | pipe.OfIter[Tuple[Tuple[bool, str, str, str], Tuple[bool, str, str, str]]]
                    .map(
                        pipe.tup2_unpack(lambda tup_before, tup_after: 
                            construct_self(tup_before, tup_after)
                        ))
                    | pipe.OfIter[SyncCommandsImpl.Rename]
                    .to_list()
            )

            return Ok(results)

        def process_rename(
            self, 
            table_columns: List[SqliteDumpParser.TableColumnsData], 
            inserts: List[SqliteDumpParser.InsertData], 
            ) -> Tuple[List[SqliteDumpParser.TableColumnsData], List[SqliteDumpParser.InsertData]]:

            new_foreign_key_defns_per_table = (
                table_columns
                    | pipe.OfIter[SqliteDumpParser.TableColumnsData]
                    .map(lambda data:
                        data.foreign_key_defns
                            | pipe.OfIter[Tuple[str, Tuple[str, str]]]
                            .map(pipe.tup2_right_tup2_flatten)
                            # | pipe.OfIter[Tuple[str, str, str]]
                            # .echo_each(pipe.tup3_unpack(lambda from_id, to_table, to_col: print(from_id, to_table, to_col, 'SELF', self.table, self.before.split(' ')[0], self.after.split(' ')[0], )))
                            | pipe.OfIter[Tuple[str, str, str]]
                            .map(pipe.tup3_unpack(lambda from_id, to_table, to_col:
                                (from_id, (self.after, to_col))
                                    if (self.entity_type == SyncCommandsImpl.RenameEntities.RENAME_TABLE and
                                        to_table == self.table) else
                                (from_id, (to_table, self.after.split(' ')[0]))
                                    if (self.entity_type ==  SyncCommandsImpl.RenameEntities.RENAME_COLUMN and
                                        to_table == self.table and
                                        to_col == self.before.split(' ')[0]) else
                                (from_id, (to_table, to_col))
                            ))
                            | pipe.OfIter[Tuple[str, Tuple[str, str]]]
                            .to_list()
                    )
                    | pipe.OfIter[List[Tuple[str, Tuple[str, str]]]]
                    .to_list()
            )

            if self.entity_type == SyncCommandsImpl.RenameEntities.RENAME_COLUMN:
                new_table_columns = (
                    table_columns
                        | pipe.OfIter[SqliteDumpParser.TableColumnsData]
                        .enumerate()
                        | pipe.OfIter[Tuple[int, SqliteDumpParser.TableColumnsData]]
                        .map(pipe.tup2_unpack(lambda i, data:
                            SqliteDumpParser.TableColumnsData(
                                data.name, 
                                data.varnames
                                    | pipe.OfIter[str]
                                    .map(lambda varname: 
                                        self.after.split(' ')[0]
                                            if varname == self.before.split(' ')[0] else 
                                        varname
                                    )
                                    | pipe.OfIter[str]
                                    .to_list()
                                , 
                                data.vartypes, 
                                data.varsettings, 
                                new_foreign_key_defns_per_table[i])
                                if data.name == self.table else
                            SqliteDumpParser.TableColumnsData(
                                data.name, data.varnames, data.vartypes, data.varsettings, new_foreign_key_defns_per_table[i])
                        ))
                        | pipe.OfIter[SqliteDumpParser.TableColumnsData]
                        .to_list()
                )
                # Changing column name has no effect on insert statements
                new_inserts = inserts
            elif self.entity_type == SyncCommandsImpl.RenameEntities.RENAME_TABLE:
                new_table_columns = (
                    table_columns
                        | pipe.OfIter[SqliteDumpParser.TableColumnsData]
                        .enumerate()
                        | pipe.OfIter[Tuple[int, SqliteDumpParser.TableColumnsData]]
                        .map(pipe.tup2_unpack(lambda i, data:
                            SqliteDumpParser.TableColumnsData(
                                self.after, 
                                data.varnames, 
                                data.vartypes, 
                                data.varsettings, 
                                new_foreign_key_defns_per_table[i])
                                if data.name == self.table else
                            SqliteDumpParser.TableColumnsData(
                                data.name, data.varnames, data.vartypes, data.varsettings, new_foreign_key_defns_per_table[i])
                        ))
                        | pipe.OfIter[SqliteDumpParser.TableColumnsData]
                        .to_list()
                )

                new_inserts = (
                    inserts
                        | pipe.OfIter[SqliteDumpParser.InsertData]
                        .map(lambda insert:
                            SqliteDumpParser.InsertData(self.after, insert.values)
                                if insert.table_name == self.table else
                            insert
                        )
                        | pipe.OfIter[SqliteDumpParser.InsertData]
                        .to_list()
                )
            else:
                assert False, 'unreachable' 
            
            return (new_table_columns, new_inserts)
    
    # class ImplFromStrForRename(FromStr, metaclass=Impl, target="SyncCommandsImpl.Rename"):
    #     pass

SyncCommands = Union[
    SyncCommandsImpl.Rename
]

def verify_path_no_folders_and_has_correct_ext(path: str, valid_ext: str, must_exist: bool=True) -> Result[str, str]:
    ext = os.path.splitext(path)[1]
    dir = os.path.split(path)[0]

    return (
        Err(f'Invalid extension for {path}. Extension must be {valid_ext}.') 
            if ext != valid_ext else
        Err('Invalid directory. Please specify a file in the specified mountpoint with no folders') 
            if dir != '' else
        Err(f'{path} does not exist') 
            if must_exist and os.path.exists(path) else
        Ok(path)
    )


def get_and_verify_db_filename(args: argparse.Namespace, key: str, must_exist: bool=False) -> Result[str, str]:
    """
        Ensures that `key` is a path that ends with *.db and contains no directories
    """
    return verify_path_no_folders_and_has_correct_ext(getattr(args, key), '.db', must_exist)


def get_and_verify_dbml_schema_filename(args: argparse.Namespace, key: str, must_exist: bool=True) -> Result[str, str]:
    return verify_path_no_folders_and_has_correct_ext(getattr(args, key), '.dbml', must_exist)


def unwrap_or_print_and_exit_on_app_err(result: Result[T, Error], exit_code: int=1) -> IO[T]:
    if result.is_ok():
        return IO(result.unwrap())
    else:
        err = result.unwrap_err()
        print(f'Error: type: {err.errno}, data: {err.data}, details: {err.details}')
        exit(exit_code)


def verify_schema_and_db_and_preproc_schema(
        args: argparse.Namespace, 
        schema_arg: str, 
        db_arg: str, 
        writing_to_db: bool=True) -> IO[Tuple[str, str, str, str]]:
    """
    Verifies that dbml schema and db file match conditions of being where they are and having correct
    extensions. If we're writing to the db file, then deletes the file.

    :returns: (
        dbml_schema_basename : str      This is the basename used for all the files
        dbml_schema_filename : str      Name of the verified dbml schema file
        dbm_schema_proc_filename : str  Name of the preprocessed dbml schema file
        db_filename : str               Name of the verified db file
    )
    """
    dbml_schema_filename = (
        get_and_verify_dbml_schema_filename(args, schema_arg, must_exist=True)
            .unwrap()
    )
    dbml_schema_basename = os.path.splitext(dbml_schema_filename)[0]

    db_filename = (
        dbml_schema_basename + '.db'
            if getattr(args, db_arg) == None else
        get_and_verify_db_filename(args, db_arg, must_exist=not writing_to_db)
            .unwrap()
    )

    dbml_schema_proc_filename = f'{MOUNT_PATH}{dbml_schema_basename}.proc.dbml'

    # First, we apply the cpp preprocessor. This allows for file includes as well as type rename defines.
    os_system(f'cpp {MOUNT_PATH}{dbml_schema_filename} -P -o {dbml_schema_proc_filename}')

    # Remove the output file if it exists
    if writing_to_db:
        if os.path.exists(MOUNT_PATH + db_filename):
            os.remove(MOUNT_PATH + db_filename)
    
    return IO((dbml_schema_basename, dbml_schema_filename, dbml_schema_proc_filename, db_filename))


def produce_db_from_data_dir(datadir: str, output_db_filename: str) -> IO[None]:
    # Let's first get the base db inserts so that we can avoid unnecessary unique constraint issues
    _, base_inserts = (
        gen_and_parse_sqlite3_dump_of_db(output_db_filename)
            .extract(caller_returns_IO=True)
    )

    inserts = TableCsvDir.import_directory(datadir) \
        .extract(caller_returns_IO=True)

    # Let's make sure to remove from inserts any identical inserts in base_inserts
    def insert_in_base(base_inserts: List[SqliteDumpParser.InsertData], 
                        insert: SqliteDumpParser.InsertData) -> bool:
        for base_insert in base_inserts:
            if insert.table_name != base_insert.table_name:
                continue
            if len(insert.values) != len(base_insert.values):
                continue

            is_identical = True # check to falsify
            for i in range(len(insert.values)):
                if insert.values[i].strip() != base_insert.values[i].strip():
                    is_identical = False
            if is_identical:
                return True
        return False

    inserts = (
        inserts
            | pipe.OfIter[SqliteDumpParser.InsertData]
            .filter(lambda ins: not insert_in_base(base_inserts, ins))
            | pipe.OfIter[SqliteDumpParser.InsertData]
            .to_list()
    )

    insert_dump_sql = SqliteDumpParser.create_insert_dump(inserts)

    with open(f'{MOUNT_PATH}dump.sql', 'w') as fw:
        fw.write(insert_dump_sql)

    os_system(f'sqlite3 {MOUNT_PATH}{output_db_filename} < {MOUNT_PATH}dump.sql')
    # os.system(f'rm {MOUNT_PATH}dump.sql')

    return IO(None)


def produce_new_db(
        table_columns: List[SqliteDumpParser.TableColumnsData], 
        inserts: List[SqliteDumpParser.InsertData], 
        dbml_schema_basename: str,
        opt_db_name_override: Optional[str]=None,
        inplace: bool=False,
        opt_datadir: Optional[str]=None) -> IO[None]:
    new_dump_content = recreate_sqlite3_dump_of_db(table_columns, inserts)
    print('\nnew_db_content\n', new_dump_content)
    new_dump_sql_path = f'{MOUNT_PATH}{dbml_schema_basename}.dump.sql'

    if opt_db_name_override:
        opt_db_basename = os.path.splitext(opt_db_name_override)[0]
    else:
        opt_db_basename = None

    if inplace:
        if opt_db_name_override:
            new_db_basename = f'{opt_db_basename}'
        else:
            new_db_basename = f'{dbml_schema_basename}'
    else:
        if opt_db_name_override:
            new_db_basename = f'{opt_db_basename}.new'
        else:
            new_db_basename = f'{dbml_schema_basename}.new'

    new_db_path = f'{MOUNT_PATH}{new_db_basename}.db'

    with open(new_dump_sql_path, 'w') as f:
        f.write(new_dump_content)

    if os.path.exists(new_db_path):
        os.remove(new_db_path)

    os_system(f'sqlite3 {new_db_path} < {new_dump_sql_path}')

    os.remove(new_dump_sql_path)

    if opt_datadir:
        _app_export_data(f'{new_db_basename}.db', opt_datadir)

    return IO(None)


def gen_and_parse_sqlite3_dump_of_db(db_filename: str) -> IO[Tuple[List[SqliteDumpParser.TableColumnsData], 
                                                                List[SqliteDumpParser.InsertData]]]:
    os_system(f'sqlite3 {MOUNT_PATH}{db_filename} ".dump" > {MOUNT_PATH}dump.sql')
    with open(f'{MOUNT_PATH}dump.sql', 'r') as fr:
        dump_str = fr.read()
    # os.remove(f'{MOUNT_PATH}dump.sql')
    table_columns = SqliteDumpParser.parse_tables_and_columns_from_dump(dump_str).unwrap()
    inserts = SqliteDumpParser.parse_inserts(dump_str).unwrap()


    return IO((table_columns, inserts))


def recreate_sqlite3_dump_of_db(table_columns: List[SqliteDumpParser.TableColumnsData], inserts: List[SqliteDumpParser.InsertData]) -> str:
    mut_result = ''

    mut_result += 'PRAGMA foreign_keys=OFF;\n'
    mut_result += 'BEGIN TRANSACTION;\n'


    for table in table_columns:
        mut_result += f'CREATE TABLE {table.name} (\n'

        for (varname, vartype, varsettings) in zip(table.varnames, table.vartypes, table.varsettings):
            mut_result += f'  {varname} {vartype} {varsettings} \n'

        for (i, (from_key, (to_key_table, to_key_col))) in enumerate(table.foreign_key_defns):
            mut_result += f'  FOREIGN KEY({from_key}) REFERENCES {to_key_table}({to_key_col})'
            if i < len(table.foreign_key_defns) - 1:
                mut_result += ','
            mut_result += '\n'

        mut_result += f');\n'


        for insert in inserts:
            if insert.table_name == table.name:
                mut_result += f'INSERT INTO {table.name} VALUES('

                for (i, value) in enumerate(insert.values):
                    mut_result += f'{value}'
                    if i < len(insert.values) - 1:
                        mut_result += f','

                mut_result += f');\n'
    mut_result += f'COMMIT;\n'

    return mut_result


def _app_export_data(db_filename: str, datadir: str) -> IO[None]:
    table_columns, inserts = (
        gen_and_parse_sqlite3_dump_of_db(db_filename)
            .extract(caller_returns_IO=True)
    )

    return TableCsvDir.export_directory(datadir, table_columns, inserts)


def app_gen(args: argparse.Namespace) -> IO[None]:
    (dbml_schema_basename, dbml_schema_filename, dbml_schema_proc_filename, output_db_filename) = (
        verify_schema_and_db_and_preproc_schema(args, 'dbml_schema_filename', 'output', writing_to_db=True)
            .extract(caller_returns_IO=True)
    )

    output_db_filename_no_ext = os.path.splitext(output_db_filename)[0]
    sql_filename = output_db_filename_no_ext + '.sql'
    
    os_system(f'dbml_sqlite {MOUNT_PATH}{dbml_schema_basename}.proc.dbml -f > {MOUNT_PATH}{sql_filename}')
    os_system(f'sqlite3 {MOUNT_PATH}{output_db_filename} < {MOUNT_PATH}{sql_filename}')

    # Let's get rid of temporary files
    if args.no_proc:
        proc_file = f'{MOUNT_PATH}{dbml_schema_basename}.proc.dbml'
        if os.path.exists(proc_file):
            os.remove(proc_file)

    if args.no_sql:
        filename = f'{MOUNT_PATH}{sql_filename}'
        if os.path.exists(filename):
            os.remove(filename)

    if args.datadir:
        produce_db_from_data_dir(args.datadir, output_db_filename)

    return IO(None)

def app_export_data(args: argparse.Namespace) -> IO[None]:
    db_filename = (
        get_and_verify_db_filename(args, 'db_filename', must_exist=True)
            | pipe.Of[Result[str, str]]
            .to(lambda res:
                unwrap_or_print_and_exit_on_err(res)
                    .extract(caller_returns_IO=True)
            )
    )

    return _app_export_data(db_filename, args.datadir)

def app_sync(args: argparse.Namespace) -> IO[None]:
    (dbml_schema_basename, dbml_schema_filename, dbml_schema_proc_filename, db_filename) = (
        verify_schema_and_db_and_preproc_schema(args, 'dbml_schema_filename', 'db', writing_to_db=False)
            .extract(caller_returns_IO=True)
    )

    db_filename_no_ext = os.path.splitext(db_filename)[0]
    sql_filename = db_filename_no_ext + '.sql'

    # Get data dump for current db
    table_columns, inserts = (
        gen_and_parse_sqlite3_dump_of_db(db_filename)
            .extract(caller_returns_IO=True)
    )

    with open(dbml_schema_proc_filename, 'r') as f:
        dbml_schema_content = f.read()

    # Parse and process rename commands
    rename_commands = (
        SyncCommandsImpl.Rename.from_str(dbml_schema_content)
            | pipe.Of[Result[List[SyncCommandsImpl.Rename], Error]]
            .to(lambda res:
                unwrap_or_print_and_exit_on_app_err(res)
                    .extract(caller_returns_IO=True)
            )
    )

    # Apply the rename to the table columns
    new_table_columns, new_inserts = (
        rename_commands
            | pipe.OfIter[SyncCommandsImpl.Rename]
            .echo_each(lambda command: print('command', command))
            | pipe.OfIter[SyncCommandsImpl.Rename]
            .fold((table_columns, inserts), lambda state, command:
                command.process_rename(*state)
            )
    )

    produce_new_db(new_table_columns, new_inserts, dbml_schema_basename, inplace=args.inplace,
                   opt_datadir=args.datadir)


    # Recreate the db using the new dump


    # Parse sync commands 

    # TODO check for column reorders

    # TODO check for column additions

    # TODO check for column removes
    
    # Verify sync commands can be applied

    # 


    return IO(None)

def app_export_validator(args: argparse.Namespace) -> IO[None]:
    print('export_validator: Not implemented')

    return IO(None)


def main(args: argparse.Namespace) -> IO[None]:
    # print(args)

    if args.subcommand == 'gen':
        app_gen(args)
    if args.subcommand == 'export_data':
        app_export_data(args)
    if args.subcommand == 'sync':
        app_sync(args)
    if args.subcommand == 'export_validator':
        app_export_validator(args)
    
    return IO(None)


def parse_cmdline_args() -> argparse.Namespace:
    desc = '''
    Tool to interface with sqlite3 database files. Creates a new db from a dbml file schema
    (See https://docs.dbdiagram.io/dbml) and also is able to export and import table rows into
    the db file.
    '''
    # Make parser object
    p = argparse.ArgumentParser(prog='dbmint', description=desc,
        formatter_class=argparse.RawDescriptionHelpFormatter)
    
    p.add_argument('-v', '--verbose', action='count', default=0,
                   help="Increase verbosity level (use -v, -vv, or -vvv)")
                   
    subparsers = p.add_subparsers(dest='subcommand')
    subparsers.required = True

    sp = subparsers.add_parser('gen', 
                               help='Generates a new db given a dbml schema file')
    sp.add_argument("dbml_schema_filename",
                    help="The name of the schema.dbml file to generate a db from")
    sp.add_argument('-o', '--output', type=str, required=False, 
                    help='The name of the file to output the *.db to. No directory is allowed. Defaults to same name as schema.')
    sp.add_argument('--no-sql', required=False, action='store_true',
                    help='Omits outputting the generated sql file that was used to created the db')
    sp.add_argument('--no-proc', required=False, action='store_true',
                    help='Omits outputting the generated proc dbml file that was used to created the db')
    sp.add_argument('-d', '--datadir', type=str, required=False, 
                    help='Data to intialize db. A directory that contains table row csv files. Check command export_data to generate this from a db')

    sp = subparsers.add_parser('export_data', 
                               help='Exports data added to a sqlite3 db')
    sp.add_argument("db_filename",
                    help="The *.db file to use to check for data in")
    sp.add_argument('-d', '--datadir', type=str, required=True, 
                    help='The name of the dir to output table rows to. Will create if non-existent.')

    sp = subparsers.add_parser('sync', 
                               help='Amends a db and schema given sync comment commands in the schema. See docs/sync.md')
    sp.add_argument("dbml_schema_filename",
                    help="The name of the schema.dbml file to generate a db from")
    sp.add_argument("--db", type=str, required=False, 
                    help="The name of the db file to amend content of. Assumes same name as schema otherwise.")
    sp.add_argument('-d', "--datadir", type=str, required=False, 
                    help="The name of the directory of exported csvs to amend or create")
    sp.add_argument('-i', '--inplace', required=False, action='store_true',
                    help='Replaces the db or exported directory as-is. If not, it creates a *.new.db, etc.')


    sp = subparsers.add_parser('export_validator', 
                               help='Exports validator according to the custom EXT command checks in the schema.dbml')
    sp.add_argument("dbml_schema_filename",
                    help="The name of the schema.dbml file to generate a validator for")
    sp.add_argument('-f', '--format', type=str, required=True, 
                    choices=['Python', 'GDScript'],
                    help='The acceptable Format. Currently only supports: Python, GDScript')

    sp = subparsers.add_parser('unittest', 
                    help='run the unit tests instead of main')

    return(p.parse_args())

def _main() -> IO[None]:
    if sys.version_info<(3,5,0):
        sys.stderr.write("You need python 3.5 or later to run this script\n")
        sys.exit(1)

    # if you have unittest as part of the script, you can forward to it this way
    if len(sys.argv) >= 2 and sys.argv[1] == 'unittest':
        import unittest
        sys.argv[0] += ' unittest'
        sys.argv.remove('unittest')
        print(sys.argv)
        unittest.main()
        exit(0)

    args = parse_cmdline_args()
    return main(args)


import unittest
class Module1UnitTests(unittest.TestCase):
   def test_something(self) -> None:
       self.assertTrue(True, "rigorous test :)")

   def test_prototype(self) -> None:
       pass
       # out = subprocess.check_output('timetrap -v d', shell=True)
       # os.system('timetrap -v d')

class Module2UnitTests(unittest.TestCase):
   def test_something(self) -> None:
       self.assertTrue(True, "rigorous test :)")


if __name__ == '__main__':
    _main()
