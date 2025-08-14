- [1. On versioning](#1-on-versioning)
- [2. Rust project autogeneration](#2-rust-project-autogeneration)
  - [PEND](#pend)
- [3. Python Commands](#3-python-commands)
  - [3.1. Sync](#31-sync)
    - [3.1.1. Options](#311-options)
    - [3.1.2. Commands](#312-commands)
      - [3.1.2.1. Rename Table or column](#3121-rename-table-or-column)
      - [3.1.2.2. Deleting Table or column (TODO)](#3122-deleting-table-or-column-todo)
      - [3.1.2.3. Adding new columns (TODO)](#3123-adding-new-columns-todo)
      - [3.1.2.4. Reordering columns in Table (TODO)](#3124-reordering-columns-in-table-todo)
  - [PEND](#pend-1)
- [4. Concepts](#4-concepts)

# 1. On versioning
- This specification is marked `v1.0.0t` with a `t` in the end for TODO. This means it's undergoing much construction work and not yet stabilized.

# 2. Rust project autogeneration

## PEND

# 3. Python Commands

## 3.1. Sync

We might want to make amends to our databases through the schema file. 
some changes are auto-detected, while others require some diff information like renames.

### 3.1.1. Options
- TODO Use --inplace to replace the db in-place. Otherwise, this will create a new `*.new.db` file
- TODO To additionally amend a directory of csvs, use `--exported`

### 3.1.2. Commands

#### 3.1.2.1. Rename Table or column
- We can rename columns and tables as in the following:
```
    Table Persons {
        id integer [pk]
        name varchar
    }
```

This updates the Table `Persons` to `Customers`
```diff
-   Table Persons {
    Table Customers {
        id integer [pk]
        name varchar
    }
```

This updates the column `name` to `username`
```diff
    Table Person {
        id integer [pk]
-       name varchar
        username varchar
    }
```

#### 3.1.2.2. Deleting Table or column (TODO)
- When deleting a column or a table, it will be detected by a comparison for the given db's schema
- Note that all dependencies of the item will be deleted in terms of foreign keys.

#### 3.1.2.3. Adding new columns (TODO)
- Simply add new columns.

#### 3.1.2.4. Reordering columns in Table (TODO)
- Simply reorder the columns.

## PEND

# 4. Concepts