from dataclasses import dataclass
import sys
import os
import argparse
import datetime
import enum
import pipe
from result import Result, Ok, Err
from typing import Dict, Tuple, List, TypedDict, Union
import unittest

MOUNT_PATH = '/mnt/'

class ErrorType(enum.Enum):
    SqliteDumpParseError = enum.auto()
    TableCsvDirError = enum.auto()


Errno = Union[
    'SqliteDumpParser.Errno',
    'TableCsvDir.Errno'
]


class Error:
    def __init__(self, type: ErrorType, errno: Errno, details: str='', data: dict={}):
        self.type = type
        self.errno = errno
        self.details = details
        self.data = data
    
    def __str__(self):
        return str(self.__dict__)


class SqliteDumpParser:
    """
    Able to parse minimum table data and row inserts from "sqlite3 <db_file> ".dump"
    """

    class Errno(enum.Enum):
        TableParseError = enum.auto()
        InsertParseError = enum.auto()

    class TableColumnsData:
        def __init__(self, name: str, varnames: List[str], vartypes: List[str]):
            self.name = name
            self.varnames = varnames
            self.vartypes = vartypes

        def __str__(self):
            return str(self.__dict__)

    class InsertData:
        def __init__(self, table_name: str, values: List[str]):
            self.table_name = table_name
            self.values = values
        
        def __str__(self):
            return str(self.__dict__)

    @staticmethod
    def parse_tables_and_columns(s: str) -> Result[List[TableColumnsData], Error]:
        parsing_table = False

        lines = s.splitlines()

        results = []
        table_name = ''
        varnames = []
        vartypes = []

        for line in lines:
            line = line.strip()

            if not parsing_table:
                table_syntax_start = 'CREATE TABLE '
                table_syntax_end = '('
                if line.startswith(table_syntax_start) and line.endswith(table_syntax_end):
                    line = line[len(table_syntax_start):-len(table_syntax_end)]
                    table_name = line.strip()
                    parsing_table = True
                    continue

            if parsing_table:
                if line == ');':
                    parsing_table = False
                    if len(varnames) != 0 and len(vartypes) != 0 and table_name != '':
                        if len(words) < 2:
                            return Err(Error(
                                ErrorType.SqliteDumpParseError, 
                                SqliteDumpParseErrno.TableParseError, 
                                f'varnames and vartypes of different len: {len(varnames)} != {len(vartypes)} for line {line}'))

                        results.append(SqliteDumpParser.TableColumnsData(table_name, varnames, vartypes))
                        table_name = ''
                        varnames = []
                        vartypes = []
                    continue

                words = list(line.split(' '))
                if len(words) < 2:
                    return Err(Error(
                        ErrorType.SqliteDumpParseError, 
                        SqliteDumpParseErrno.TableParseError, 
                        f'need at least varname vartype in line {line}'))
                
                # We are not interested in key link lines and other non-member specifying lines
                if words[0] in 'FOREIGN':
                    continue

                varnames.append(words[0])
                vartypes.append(words[1])
                continue
        
        return Ok(results)

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
                        ErrorType.SqliteDumpParseError, 
                        SqliteDumpParser.Errno.InsertParseError, 
                        f'Expected VALUES(...) in line {line}'))

                line = line[len(prefix):]
                words = line.split(' ')
                table_name = words[0]
                values = line[line.find('(')+1:line.find(')')]
                values = list(values.split(','))
                values = values | pipe.map(lambda s: s.strip()) | pipe.Pipe(list)

                result.append(SqliteDumpParser.InsertData(table_name, values))

        return Ok(result)

    def create_insert_dump(inserts: List['SqliteDumpParser.InsertData']) -> str:
        sql_script = ''

        for insert in inserts:
            values_str = ','.join(insert.values | pipe.map(lambda s: s.strip()) | pipe.Pipe(list))
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
                    inserts: List[SqliteDumpParser.InsertData]) -> Result[None, Error]:
        table_name = table_columns.name

        with open(f'{MOUNT_PATH}{dir_name}/{table_name}.csv', 'w') as fw:
            # Let's write the columns line first
            column_line = ''
            for column in table_columns.varnames:
                column_line += f'{column},'
            
            if not column_line.endswith(','):
                return Err(Error(
                    ErrorType.TableCsvDirError,
                    TableCsvDir.Errno.TableWithNoColumns, 
                    data={'table_columns': table_columns}))
            
            column_line = column_line[:-len(',')]

            fw.write(column_line + '\n')

            inserts = inserts | pipe.filter(lambda ins: ins.table_name == table_name) \
                              | pipe.Pipe(list)
            
            for insert in inserts:
                insert_line = ''

                if len(insert.values) != len(table_columns.varnames):
                    return Err(Error(
                        ErrorType.TableCsvDirError,
                        TableCsvDir.Errno.ValuesOfDiffLenToCols, 
                        data={'table_columns': table_columns,
                              'insert': insert}))

                for value in insert.values:
                    insert_line += f'{value},'

                # Has to be non-empty since columns have to be non-empty and 
                # it must be same count as columns
                insert_line = insert_line[:-len(',')]

                fw.write(insert_line + '\n')
        
        return Ok(())

    @staticmethod
    def export_directory(dir_name: str, tables_columns: List[SqliteDumpParser.TableColumnsData], 
                         inserts: List[SqliteDumpParser.InsertData]):
        os.system(f'mkdir -p {MOUNT_PATH}{dir_name}')

        for table_columns in tables_columns:
            res = TableCsvDir.export_csv(dir_name, table_columns, inserts)
            if res.is_err():
                print(f'Error: {res.unwrap_err()}')

    @staticmethod
    def import_csv(csv_path) -> List[SqliteDumpParser.InsertData]:
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

            values = list(line.split(','))
            values = values | pipe.map(lambda s: s.strip()) | pipe.Pipe(list)
            result.append(SqliteDumpParser.InsertData(table_name, values))
        
        return result

    @staticmethod
    def import_directory(dir_name: str) -> List[SqliteDumpParser.InsertData]:
        csv_filenames = os.listdir(f'{MOUNT_PATH}{dir_name}')
        csv_filenames = csv_filenames | pipe.filter(lambda s: s.endswith('.csv')) \
                                      | pipe.Pipe(list)

        results = []

        for csv_filename in csv_filenames:
            results.extend(TableCsvDir.import_csv(f'{MOUNT_PATH}{dir_name}/{csv_filename}'))
        
        return results


def get_valid_db_filename(args, key):
    output_filename = getattr(args, key)
    output_filename_ext = os.path.splitext(output_filename)[1]
    output_filename_dir = os.path.split(output_filename)[0]
    if output_filename_ext != '.db':
        print('Please select a *.db file for output')
        exit(1)
    if output_filename_dir != '':
        print('Please specify a file in the specified mountpoint with no folders')
        exit(1)

    return (output_filename)


def app_gen(args):
    output_filename = get_valid_db_filename(args, 'output')
    output_filename_no_ext = os.path.splitext(output_filename)[0]
    sql_filename = output_filename_no_ext + '.sql'
    
    # Remove the output file if it exists
    if os.path.exists(MOUNT_PATH + output_filename):
        os.remove(MOUNT_PATH + output_filename)

    dbml_schema_filename = args.dbml_schema_filename
    dbml_schema_filename_ext = os.path.splitext(dbml_schema_filename)[1]
    dbml_schema_filename_dir = os.path.split(dbml_schema_filename)[0]
    dbml_schema_basename = os.path.splitext(dbml_schema_filename)[0]

    if dbml_schema_filename_ext != '.dbml':
        print('Please specify a *.dbml file to generate a sqlite3 db from')
        exit(1)
    if dbml_schema_filename_dir != '':
        print('Please specify a file in the specified mountpoint with no folders')
        exit(1)

    os.system(f'cpp {MOUNT_PATH}{dbml_schema_filename} -P -o {MOUNT_PATH}{dbml_schema_basename}.proc.dbml')
    os.system(f'dbml_sqlite {MOUNT_PATH}{dbml_schema_basename}.proc.dbml -f > {MOUNT_PATH}{sql_filename}')
    os.system(f'sqlite3 {MOUNT_PATH}{output_filename} < {MOUNT_PATH}{sql_filename}')

    # Let's get rid of temporary files
    proc_file = f'{MOUNT_PATH}{dbml_schema_basename}.proc.dbml'
    if os.path.exists(proc_file):
        os.remove(proc_file)

    if args.no_sql:
        filename = f'{MOUNT_PATH}{sql_filename}'
        if os.path.exists(filename):
            os.remove(filename)

    if args.datadir:
        # Let's first get the base db inserts so that we can avoid unnecessary unique constraint issues
        _, base_inserts = gen_and_parse_sqlite3_dump_of_db(output_filename)

        inserts = TableCsvDir.import_directory(args.datadir)

        # Let's make sure to remove from inserts any identical inserts in base_inserts
        def insert_in_base(base_inserts: List[SqliteDumpParser.InsertData], 
                           insert: SqliteDumpParser.InsertData):
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

        inserts = inserts | pipe.filter(lambda ins: not insert_in_base(base_inserts, ins))

        insert_dump_sql = SqliteDumpParser.create_insert_dump(inserts)

        with open(f'{MOUNT_PATH}dump.sql', 'w') as fw:
            fw.write(insert_dump_sql)

        os.system(f'sqlite3 {MOUNT_PATH}{output_filename} < {MOUNT_PATH}dump.sql')
        # os.system(f'rm {MOUNT_PATH}dump.sql')


def gen_and_parse_sqlite3_dump_of_db(db_filename: str) -> Tuple[List[SqliteDumpParser.TableColumnsData], 
                                                           List[SqliteDumpParser.InsertData]]:
    os.system(f'sqlite3 {MOUNT_PATH}{db_filename} ".dump" > {MOUNT_PATH}dump.sql')
    with open(f'{MOUNT_PATH}dump.sql', 'r') as fr:
        dump_str = fr.read()
    os.remove(f'{MOUNT_PATH}dump.sql')
    table_columns = SqliteDumpParser.parse_tables_and_columns(dump_str).unwrap()
    inserts = SqliteDumpParser.parse_inserts(dump_str).unwrap()

    return (table_columns, inserts)


def _app_export_data(db_filename, datadir):
    table_columns, inserts = gen_and_parse_sqlite3_dump_of_db(db_filename)

    TableCsvDir.export_directory(datadir, table_columns, inserts)


def app_export_data(args):
    db_filename = get_valid_db_filename(args, 'db_filename')
    _app_export_data(db_filename, args.datadir)

def app_export_validator(args):
    print('export_validator: Not implemented')


def main(args: argparse.Namespace):
    # print(args)

    if args.subcommand == 'gen':
        app_gen(args)
    if args.subcommand == 'export_data':
        app_export_data(args)
    if args.subcommand == 'export_validator':
        app_export_validator(args)


def cmdline_args():
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
    sp.add_argument('-o', '--output', type=str, required=True, 
                    help='The name of the file to output the *.db to')
    sp.add_argument('--no-sql', required=False, action='store_true',
                    help='Omits outputting the generated sql file that was used to created the db')
    sp.add_argument('-d', '--datadir', type=str, required=False, 
                    help='A directory that contains table row csv files. Check command export_data to generate this from a db')

    sp = subparsers.add_parser('export_data', 
                               help='Exports data added to a sqlite3 db')
    sp.add_argument("db_filename",
                    help="The *.db file to use to check for data in")
    sp.add_argument('-d', '--datadir', type=str, required=True, 
                    help='The name of the dir to output table rows to. Will create if non-existent.')

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

def _main():
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

    args = cmdline_args()
    main(args)


import unittest
class Module1UnitTests(unittest.TestCase):
   def test_something(self):
       self.assertTrue(True, "rigorous test :)")

   def test_prototype(self):
       pass
       # out = subprocess.check_output('timetrap -v d', shell=True)
       # os.system('timetrap -v d')

class Module2UnitTests(unittest.TestCase):
   def test_something(self):
       self.assertTrue(True, "rigorous test :)")


if __name__ == '__main__':
    _main()
