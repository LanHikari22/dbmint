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

- [Intro](#intro)
- [See also](#see-also)
- [Getting Started](#getting-started)
- [Features](#features)
- [Caveats](#caveats)
- [Use Cases](#use-cases)
  - [Generating a Sqlite3 database file (.db) from a schema file (.dbml)](#generating-a-sqlite3-database-file-db-from-a-schema-file-dbml)
  - [Exporting database data to a directory of csvs](#exporting-database-data-to-a-directory-of-csvs)
- [Contribution](#contribution)
- [Support](#support)
- [License](#license)
- [Credits](#credits)

# Intro

This tool aims to simplify the workflow of generating sqlite3 databases. It uses the [DBML][5] file format for schema generation to 
make creating schemas easy and to also benefit from its graphical editor functionality for easier diagram generation! It also allows exchangability between
database content and a directory of csv files.

# See also
- The project on [Github][1], and [Dockerhub][6].
- The project [notes][7].
- The project [documentation](docs/intro.md).

# Getting Started
You will need access to Docker, otherwise there is no setup necessary. See [their install instructions.][4]

# Features
- Minimal install. Tool can be used through a single docker command.
- Easily export the data of a db to a directory of table csvs which can be imported back in.
- Allows use of the #defines from the C preprocessor on the .dbml file format in order to avoid duplications.
- TODO upcoming, generate a database library in Rust based on dbmt files

# Caveats
- Currently supports one to one relations only, although the dbml file format is capable of describing one to many, and many to many. This may be supported in
  the future with the ability to automatically many to many (MTM) or one to many (OTM) tables from the schema with external validation checks.
 
# Use Cases

## Generating a Sqlite3 database file (.db) from a schema file (.dbml)

Write the following schema.db to a file. This is mostly identical to the example in [dbdiagram.io][3] except with modifications for 
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
Table user_owns_posts {
  id integer [primary key]
  user_id integer [ref: - users.id]
  post_id integer [ref: - posts.id]
}

```

| ![Image](./attachments/Screenshot%20From%202025-08-15%2006-58-26.png)
:--
| Diagram generated from [dbdiagram.io](https://dbdiagram.io/d) using the above code |

Now let's generate a sqlite3 db file using the above schema.dbml (On Windows CMD, you need to remove the "--user $(id -u):$(id -g)" bit.)
```
  docker run --rm -it -v .:/mnt/ --user $(id -u):$(id -g) lan22h/dbmint:latest \
    gen schema.dbml -o mydb.db
```
This should generate a new mydb.db for us in the same directory, which has the same schema as described in schema.dbml.

It also generates a .sql from the dbml schema for the user's information. If this is not desired, use the --no-sql option for the gen subcommand:
```
  docker run --rm -it -v .:/mnt/ --user $(id -u):$(id -g) lan22h/dbmint:latest \
    gen schema.dbml -o mydb.db --no-sql
```


## Exporting database data to a directory of csvs
Now that we have a database like mydb.db, we can edit this with any SQLite3 client like [DbGate][2]. 

Let's extract the content of the db with
the following command:

```
  docker run --rm -it -v .:/mnt/ --user $(id -u):$(id -g) lan22h/dbmint:latest \
    export_data mydb.db -d mydata/
```

Now this generates a directory `mydata/` with csv files for each table in our db. We can modify the csv files directly
and then import this back into the db:

```
  docker run --rm -it -v .:/mnt/ --user $(id -u):$(id -g) lan22h/dbmint:latest \
    gen schema.dbml -d mydata/ -o mydb.db 
```

# Contribution
All contribution and feature requests are welcome. Please raise an issue and we can talk about anything.

# Support

If this project brings value to you, please consider supporting me with a monthly subscription or [buying me a coffee](https://buymeacoffee.com/lan22h).

Your support would greatly help me to create new tools or improve existing ones!

# License
This theme is licensed under the [MIT license](https://opensource.org/licenses/mit-license.php) © 2025 Mohammed Alzakariya.

# Credits
- Thanks to the [dbml file format][8] creators for making this possible.
- Thanks to the [pydbml][10] creators for providing dbml parsing in python.
- Thanks to the [dbml-sqlite][9] creator for extending dbml support to sqlite.

[1]: https://github.com/LanHikari22/dbmint
[2]: https://dbgate.org/
[3]: https://dbdiagram.io/d
[4]: https://docs.docker.com/engine/install/
[5]: https://dbdiagram.io/
[6]: https://hub.docker.com/repository/docker/lan22h/dbmint/general
[7]: https://github.com/delta-domain-rnd/delta-trace/blob/webview/lan/projects/2025/000%20dbmint/docs/2025/000%20Dbmint.md
[8]: https://github.com/holistics/dbml
[9]: https://pypi.org/project/dbml-sqlite/
[10]: https://github.com/Vanderhoof/PyDBML