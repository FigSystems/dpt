# Introduction

This document outlines the fpkg method of distributing software. This document stresses 3 the points of

- Reproducibility: I should be able to give my package to a friend and it should work exactly the same as it did on my system.

- Forward compatible: My friend should be able to run that exact same package 10 years later with no trouble.

- Stability: I should be able to install many combinations of packages and not have anything break. Further, if something does break, I should be able to easily roll back.

Reproducability could be defined as consistent outcomes. When a software developer releases a package, they want it to work on all system configurations. Conventional tools fail on this point as the developer have no idea what configuration their users may have on their system. fpkg excels in this area, however, as all applications have a well defined environment that the developer controls, unaffected by the user’s configuration.

Stability is another important topic. Any package manager should be stable, and never require breaking another package to install another. Dependency hell is an unfortunate result from conventional solutions, but most package management systems don’t care about fixing it, instead they just “freeze” packages at a known point in time. This is sad.

There are a bunch of terms and ideas used in this document:

- Package pool: The “data” of the package manager. Directories containing the packages themselves. There is currently only one package pool on a given system, generally located it /fpkg/pool.

- Environment: A directory containing all the (symlinks to) a package’s, and its dependency’s, files.

- Package: A single package, often located in an fpkg pool.

- Repository: A location on the internet or locally that provides packages to fpkg.

# Command line usage

Covers the basics of fpkg’s command line usage.

## fpkg install/add [package(s)]

Installs package(s) into the pool.

## fpkg rm/uninstall [package(s)]

Removes package(s) from the pool.

## fpkg gen-pkg [directory]

Generates a package from the directory given. More detail later.

## fpkg run [program]

Runs a program inside its directory.

## fpkg build-env [package]

Builds, or rebuilds, the environment for the specified package.

## fpkg update

Updates the packages currently installed in the system

# Inner details

Covers the inner and implementation details of fpkg.

## The fpkg pool

The fpkg pool are composed of many directories with names following the pattern package-name-1.2.3. The main pool is stored at /fpkg/pool by default, but the location of this pool can be changed by placing the directory name in a file at `/etc/fpkg/pool`.

_Example fpkg pool_

```
/fpkg/pool
├── example-1.2.3
│    ├── usr
│    │    └── bin
│    │        └── example
│    └── ...
├── random-lib-4.5.6
│   ├── usr
│   │    └── lib
│   │       └── librandom.so
│   └── ...
└── ...
```

## Package environments

For each package, when it is installed, an environment is created. Each environment consists of symlinks to the main files inside the package and it’s dependencies. The default environment directory is /fpkg/env but this can be changed by placing a directory name inside a file at the path `/etc/fpkg/env`. Each sub-directory under the environment directory will have a package with the same name, or rather, the environment has the same name as the package it represents.

# Packages

This section defines various properties of packages, as well as their creation.

## Directory Structure

The directory structure of an fpkg is quite basic, consisting of an FHS compliant directory along with some fpkg metadata. An example package is given below:

```
example-1.2.3

├── fpkg
│    └── pkg.kdl # Package details. You write this.
└── usr
    ├── bin
    │    └── example
    └── share
        └── example
            └── example.txt
```

Most fpkgs are distributed as .fpkg files. A .fpkg file is just a compressed tar archive containing the fpkg.

## Generating packages

The process of generating packages requires writing a description file, installing the program to a directory, and then running fpkg gen-pkg [directory]. An example package description file:

```
# pkg-directory/fpkg/pkg.kdl
name example
version "1.2.3"
developer "ExampleSoft Inc."

depends python {
	version "^3.12-5"
}

depends coreutils
depends love
depends joy
depends community
...
```

Version ranges are specified in the format of

- A `^` e.g. `^2.3.4` means `>=2.3.4` and `<3.0.0`
- A `~` e.g. `^1.2.3` means `>=1.2.3` and `<1.3.0`
- No prefix requires the exact version specified

## Repository Format

Repositories are simply http(s) servers with a predefined file structure as follows:

- index.kdl: KDL file with a list of packages and package versions that are contained in this repository.

- \*.fpkg: All of the compressed fpkgs on this repository.

index.kdl is made of bunch of package nodes. In each node there is a name value, a version value, and a path value. E.g.

```
package name=python version="3.10.2" path="/python-3.10.2.fpkg" {
    depends ...
    depends ...
    ... // A copy of the package's depends section
}
```

The list of repositories is stored in `/etc/fpkg/repos` in the format of

```
https://pkg.repo/fpkg
https://another.repo
```

The repository's priorities decrease down the file i.e. The first repository has more priority then the second, and the second has more priority then the third etc.

# Dependency resolving

For dependency resolving, fpkg uses [PubGrub](https://crates.io/crates/pubgrub) due to it’s efficient and accurate dependency resolution. When one runs fpkg update what happens is

- Fpkg searches the pool and discovers currently installed applications.

- For each of the installed packages, fpkg

  - Will perform dependency resolving (PubGrub) on those packages and reports any errors, to obtain that packages dependency tree.

  - With the freshly obtained list of packages, fpkg fetches them and installs them in the pool.

  - Fpkg refreshes each of the installed package’s environments.
