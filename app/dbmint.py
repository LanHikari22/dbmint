import sys
import os
import argparse
import datetime
from result import Result, Ok, Err
from typing import Dict, Tuple, List
import unittest

MOUNT_PATH = '/mnt/'

def app_gen(args):
    output_filename = args.output
    output_filename_ext = os.path.splitext(output_filename)[1]
    output_filename_dir = os.path.split(output_filename)[0]
    if output_filename_ext != '.db':
        print('Please select a *.db file for output')
        exit(1)
    if output_filename_dir != '':
        print('Please specify a file in the specified mountpoint with no folders')
        exit(1)
    
    # Remove the output file if it exists
    if os.path.exists(MOUNT_PATH + output_filename):
        os.remove(MOUNT_PATH + output_filename)

    dbml_schema_filename = args.dbml_schema_filename
    dbml_schema_filename_ext = os.path.splitext(dbml_schema_filename)[1]
    dbml_schema_filename_dir = os.path.split(dbml_schema_filename)[0]
    dbml_schema_basename = os.path.splitext(dbml_schema_filename)[0]
    print('fuuull', dbml_schema_filename)
    print('baaase', os.path.splitext(dbml_schema_filename))

    if dbml_schema_filename_ext != '.dbml':
        print('Please specify a *.dbml file to generate a sqlite3 db from')
        exit(1)
    if dbml_schema_filename_dir != '':
        print('Please specify a file in the specified mountpoint with no folders')
        exit(1)

    os.system(f'cpp {MOUNT_PATH}{dbml_schema_filename} -P -o {MOUNT_PATH}{dbml_schema_basename}.proc.dbml')
    os.system(f'dbml_sqlite {MOUNT_PATH}{dbml_schema_basename}.proc.dbml -f > {MOUNT_PATH}schema.sqlite')
    os.system(f'sqlite3 {MOUNT_PATH}{output_filename} < {MOUNT_PATH}schema.sqlite')

    # Let's get rid of temporary files
    proc_file = f'{MOUNT_PATH}{dbml_schema_basename}.proc.dbml'
    if os.path.exists(proc_file):
        os.remove(proc_file)

    # sqlite_file = f'{MOUNT_PATH}schema.sqlite'
    # if os.path.exists(sqlite_file):
    #     os.remove(sqlite_file)

    # TODO: Handle parsing a JSON file for the input data to put into the db


def app_export_data(args):
    print('export_data: Not implemented')


def app_export_validator(args):
    print('export_validator: Not implemented')


def main(args: argparse.Namespace):
    print(args)

    if args.subcommand == 'gen':
        app_gen(args)
    if args.subcommand == 'export_data':
        app_export_data(args)
    if args.subcommand == 'export_validator':
        app_export_validator(args)


def cmdline_args():
    # Make parser object
    p = argparse.ArgumentParser(description=__doc__,
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
    sp.add_argument('-i', '--input', type=str, required=False, 
                    help='A *.json file that contains data to input into the output *.db. Check command export_data')

    sp = subparsers.add_parser('export_data', 
                               help='Exports data added to the db according to a schema')
    sp.add_argument("dbml_schema_filename",
                    help="The name of the schema.dbml file to check against")
    sp.add_argument("db_filename",
                    help="The *.db file to use to check for data in")
    sp.add_argument('-o', '--output', type=str, required=True, 
                    help='The name of the file to output the *.json to')

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
