<div align="center">
  DBMint
  <p>For a visual way to work with databases!</p>
</div>

<p align="center">
  <a href="https://opensource.org/licenses/MIT">
    <img src="https://img.shields.io/badge/License-MIT-brightgreen.svg"
      alt="License: MIT" />
  </a>
  <a href="https://buymeacoffee.com/lan22h">
    <img src="https://img.shields.io/static/v1?label=Buy me a coffee&message=%E2%9D%A4&logo=BuyMeACoffee&link=&color=greygreen"
      alt="Buy me a Coffee" />
  </a>
</p>

<div align="center">
  <sub>Built with ❤︎ by Mohammed Alzakariya</sub>
</div>

- [1. Intro](#1-intro)
- [2. See also](#2-see-also)
- [3. Getting Started](#3-getting-started)
- [4. Features](#4-features)
- [5. Caveats](#5-caveats)
- [6. TODO](#6-todo)
- [7. Use Cases](#7-use-cases)
  - [7.1. Generating a Sqlite3 database file (.db) from a schema file (.dbml)](#71-generating-a-sqlite3-database-file-db-from-a-schema-file-dbml)
  - [7.2. Exporting database data to a directory of csvs](#72-exporting-database-data-to-a-directory-of-csvs)
- [8. Contribution](#8-contribution)
- [9. License](#9-license)
- [10. Credits](#10-credits)

# 1. Intro

This tool aims to simplify the workflow of generating sqlite3 databases. It uses the [DBML](https://dbdiagram.io/) file format for schema generation to 
make creating schemas easy and to also benefit from its graphical editor functionality for easier diagram generation! It also allows exchangability between
database content and a directory of csv files.

# 2. See also
- The project on [Github](https://github.com/LanHikari22/dbmint), and [Dockerhub](https://hub.docker.com/repository/docker/lan22h/dbmint/general).
- The project [notes](https://github.com/delta-domain-rnd/delta-trace/blob/webview/lan/projects/2025/000%20dbmint/docs/2025/000%20Dbmint.md).
- The project [documentation](docs/intro.md).

# 3. Getting Started
You will need access to Docker, otherwise there is no setup necessary. See [their install instructions.](https://docs.docker.com/engine/install/)

# 4. Features
- Minimal install. Tool can be used through a single docker command.
- Easily export the data of a db to a directory of table csvs which can be imported back in.
- Allows use of the #defines from the C preprocessor on the .dbml file format in order to avoid duplications.
- TODO Extends the .dbml standard to allow for external type validations through generating an external validator script. Currently supports Python and GDScript.
- TODO Can set validation constraints on `#define`d types.

# 5. Caveats
- Currently supports one to one relations only, although the dbml file format is capable of describing one to many, and many to many. This may be supported in
  the future with the ability to automatically many to many (MTM) or one to many (OTM) tables from the schema with external validation checks.
 
# 6. TODO
- Improve diagnostics when users make errors in the dbml file
- Allow enum values to be null

# 7. Use Cases

## 7.1. Generating a Sqlite3 database file (.db) from a schema file (.dbml)

Write the following schema.db to a file. This is mostly identical to the example in [dbdiagram.io](https://dbdiagram.io/d) except with modifications for 
  one to many relations.
```
Table users {
  id integer [primary key]
  username varchar
  role varchar
  created_at timestamp
}

Table posts {
  id integer [primary key]
  title varchar
  body text [note: 'Content of the post']
  user_id integer
  status varchar
  created_at timestamp
}

Table follows {
  following_user_id integer [ref: - users.id]
  followed_user_id integer [ref: - users.id]
  created_at timestamp 
}


// A one-to-many relationship between users and posts
Table users_otm_posts {
  id integer [primary key]
  user_id integer [ref: - users.id]
  post_id integer [ref: - posts.id]
}

```

| ![Image](https://cdn.discordapp.com/attachments/1239545053752332301/1289668605927227503/image.png?ex=66f9a8fc&is=66f8577c&hm=20071a7f2300d2eb972f84c6da2c369808da3ed1cad90a5598ebfbd545004c1f&)
:--
| Diagram generated from [dbdiagram.io](https://dbdiagram.io/d) using the above code |

Now let's generate a sqlite3 db file using the above schema.dbml (On Windows CMD, you need to remove the "--user $(id -u):$(id -g)" bit.)
```
  docker run --rm -it -v .:/mnt/ --user $(id -u):$(id -g) lan22h/dbmint:latest gen schema.dbml -o mydb.db
```
This should generate a new mydb.db for us in the same directory, which has the same schema as described in schema.dbml.

It also generates a .sql from the dbml schema for the user's information. If this is not desired, use the --no-sql option for the gen subcommand:
```
  docker run --rm -it -v .:/mnt/ --user $(id -u):$(id -g) lan22h/dbmint:latest gen schema.dbml -o mydb.db --no-sql
```


## 7.2. Exporting database data to a directory of csvs
Now that we have a database like mydb.db, we can edit this with any SQLite3 client like [DbGate](https://dbgate.org/). 

Let's extract the content of the db with
the following command:

```
  docker run --rm -it -v .:/mnt/ --user $(id -u):$(id -g) lan22h/dbmint:latest export_data mydb.db -d mydata/
```

Now this generates a directory `mydata/` with csv files for each table in our db. We can modify the csv files directly
and then import this back into the db:

```
  docker run --rm -it -v .:/mnt/ --user $(id -u):$(id -g) lan22h/dbmint:latest gen schema.dbml -d mydata/ -o mydb.db 
```

# 8. Contribution
All contribution and feature requests are welcome. Please raise an issue and we can talk about anything.

# 9. License
This theme is licensed under the [MIT license](https://opensource.org/licenses/mit-license.php) © 2025 Mohammed Alzakariya.

# 10. Credits
- Thanks to the [dbml file format](https://github.com/holistics/dbml) creators for making this possible.
- Thanks to the [dbml-sqlite](https://pypi.org/project/dbml-sqlite/) creator for extending dbml support to sqlite.
