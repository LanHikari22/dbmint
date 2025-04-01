- [Intro](#intro)
- [Options](#options)
- [Commands](#commands)
  - [Rename Table or column](#rename-table-or-column)
  - [Deleting Table or column (TODO)](#deleting-table-or-column-todo)
  - [Adding new columns (TODO)](#adding-new-columns-todo)
  - [Reordering columns in Table (TODO)](#reordering-columns-in-table-todo)

# Intro
- This document describes the sync dbmint command.
- In general, we want to make amends to our databases through the schema file. 
some changes are auto-detected, while others require some diff information like renames.

# Options
- TODO Use --inplace to replace the db in-place. Otherwise, this will create a new *.new.db file
- TODO To additionally amend a directory of csvs, use --exported

# Commands

## Rename Table or column
- We can rename columns and tables as in the following:
```
    Table Persons {
        id integer [pk]
        name varchar
    }
```

This updates the Table `Persons` to `Customers`
```
-   Table Persons {
    Table Customers {
        id integer [pk]
        name varchar
    }
```

This updates the column `name` to `username`
```
    Table Person {
        id integer [pk]
-       name varchar
        username varchar
    }
```

## Deleting Table or column (TODO)
- When deleting a column or a table, it will be detected by a comparison for the given db's schema
- Note that all dependencies of the item will be deleted in terms of foreign keys.

## Adding new columns (TODO)
- Simply add new columns.

## Reordering columns in Table (TODO)
- Simply reorder the columns.