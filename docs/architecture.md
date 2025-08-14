
- [1. Design Philosophy](#1-design-philosophy)
- [2. Overview](#2-overview)
- [3. Anticipating what could scale](#3-anticipating-what-could-scale)
  - [3.1. Languages supported for library generation](#31-languages-supported-for-library-generation)
  - [3.2. Autogen features in Rust build to reach sqlite3 feature parity](#32-autogen-features-in-rust-build-to-reach-sqlite3-feature-parity)

# 1. Design Philosophy

dbmint should be seamless to use. Besides docker, there should be no installation required. Many common tasks should be possible to do in one command line.

Because this application uses a docker system, it can mix languages with ease for different purposes. In this case, the application uses both python and Rust and integrates them together. Python for front-end CLI control and general sqlite3 management, and Rust for the hard work of generating type-correct database libraries.

With time, more languages could be supported by dbmint. Being built on a docker system allows it to adapt easily with versioning and language support while remaining easily reproducible as a system.

# 2. Overview

There are currently two applications, 

A python application that mainly handles dbml->sqlite3 operations and a Rust application that handles database library generation.

The rust application is compiled native and the executable is copied when building the docker image, wheras the entire python app is currently in the image.

The application begins with the python application for CLI control and then can transition over to the rust application if needed.

New features added are typically given their own subcommands, whether in the python or rust application. As this is a CLI application, it is focused on providing specific workflows for the user.

# 3. Anticipating what could scale

## 3.1. Languages supported for library generation

Currently we only support Rust, but this list could grow quickly. The rust application may be able to handle autogeneration for other languages, but it might be that we need to use technologies from different languages. 

Care must be taken here, as the list of languages grows, so can the image size as we include new technologies. It may become necessary to tag the images being used by language to have multiple small images rather than one big image with everything in it.

## 3.2. Autogen features in Rust build to reach sqlite3 feature parity

Currently, we support a limited set of features of what sqlite3 offers for basic database manipulation. When new features are planned, they should remain consistent in convention and underlying process as the other features. We need to have a core library with which to compose new feature sets added linearly.




