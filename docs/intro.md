# Intro

You can find the project notes [here][1]. They include any task and goal planning for this project.

For an overview on the design philosophy and architectural decisions behind this project, see [architecture](./architecture.md).

This project implements its own language `*.dbmt` which is a superset language of [`*.dbml`][2]. This language adds extra features to make it easy to work with [schemas](dbmt%20specification%20v1.0.0t.md#41-schema) and break them down in production code while maintaining the intuitive visual tools used to create `dbml` schema files.

Find the `*.dbmt` language specification in [dbmt specification v1.0.0t](dbmt%20specification%20v1.0.0t.md).

Correctness of the dbmint implementation is defined in the [dbmint specification](dbmint%20specification%20v1.0.0t.md)

[1]: https://github.com/delta-domain-rnd/delta-trace/blob/webview/lan/projects/2025/000%20dbmint/docs/2025/000%20Dbmint.md
[2]: https://dbml.dbdiagram.io/docs/