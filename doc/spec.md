# Introduction

This document outlines the fpkg method of distributing software. This document stresses the points of

- Reproducibility: I should be able to give my package to a friend and it should work exactly the same as it did on my system.

- Stability: I should be able to install many combinations of packages and not have anything break. Further, if something does break, I should be able to easily roll back.

Reproducibility could be defined as consistent outcomes. When a software developer releases a package, they want it to work on all system configurations. Conventional tools fail on this point as the developer have no idea what configuration their users may have on their system. fpkg excels in this area, however, as all applications have a well defined environment that the developer controls, unaffected by the user’s configuration.

Stability is another important topic. Any package manager should be stable, and never require breaking another package to install another. Dependency hell is an unfortunate result from conventional solutions, but most package management systems don’t care about fixing it, instead they just “freeze” packages at a known point in time. This is sad.

There are a bunch of terms and ideas used in this document:

- FPKG store: The directory containing all of the packages installed in the system, located by default at `/fpkg/store` but this can be customized by placing a directory name in the file `/etc/fpkg/store`.

- Package: A single package, often located in an fpkg store. The is a directory containing the sub-directories of `data`, `env`, and `info`.

- Meta-info directory: A sub-directory of a package (`info`) containing meta info about a package. 

- Data directory: A sub-directory of a package (`data`) containing the files of a package

- Environment directory: A sub-directory of a package (`env`) containing all the symlinks to a package’s, and its dependency’s, files.

- Repository: A location on the internet or locally that provides packages to fpkg.

# Command line usage

Covers the basics of fpkg’s command line usage. Do note that fpkg NEEDS to be installed SUID.

## fpkg install/add \[package(s)\]

Installs package(s) into the store.

## fpkg rm/uninstall \[package(s)\]

Removes package(s) from the store.

## fpkg gen-pkg \[directory\]

Generates a package from the directory given. More detail later.

## fpkg run \[program\]

Runs a package.

## fpkg run-multi \[programs\]

Runs the first package specified in an environment with the other packages listed. e.g. `fpkg run-multi fish yazi musl-gcc nvim` will run `fish` in an environment that also includes `yazi`, `musl-gcc` and `nvim`.

## fpkg build-env \[package\]

Builds, or rebuilds, the environment for the specified package.

## fpkg update

Updates the packages currently installed in the system

# Inner details

Covers the inner and implementation details of fpkg.

## The fpkg store

The fpkg store are composed of many directories with names following the pattern package-name-1.2.3. The main store is located at /fpkg/store by default, but the location of this pool can be changed by placing a directory name in a file at `/etc/fpkg/pool`.

_Example fpkg store_

```
/fpkg/store
├── example-1.2.3
│    ├── data
│    ├── info
│    └── env
│    
├── random-lib-4.5.6
│    └── ...
└── ...
```

## Package environments

For each package, when it is installed, an environment is created. Each environment consists of symlinks to the main files inside the package and it’s dependencies. The environment directory for each package is located in the `env` sub-directory.

## Package meta-info directories

For each package, when it is installed, a meta info sub-directory MAY be created containing information about this package. The current meta info data that is included in this directory is as follows:

```
/fpkg/store/pkg-1.2.3/info
└── manually-installed
```

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

Most fpkgs are distributed as .fpkg files. A .fpkg file is just a zstd compressed tar archive containing the fpkg.

## Generating packages

The process of generating packages requires writing a description file, installing the program to a directory, and then running fpkg gen-pkg [directory]. An example package description file:

```
# pkg-directory/fpkg/pkg.kdl
name example
version "1.2.3"

depends python version=">=3.12"

depends coreutils
depends love
depends joy
depends community
...
```

Version ranges are specified immediately prior to the version. They can be one of the following

- `>`
- `>=`
- No prefix requires the exact version specified

### FPKGBUILDs

For convenience in the process of generating packages, one can write an FPKGUILD file, which is very similar in kind to Arch Linux's PKGBUILDs. Not all features are supported. The currently defined variables/functions in FPKGBUILDs are

- `pkgname`
- `pkgver`
- `depends`
- `build()`

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

For dependency resolving, fpkg uses [PubGrub](https://crates.io/crates/pubgrub) due to it’s efficient and accurate dependency resolution.

# Package running

When running a package, fpkg will bind mount the fpkg store directory into a temporary directory, and then symlink all of the root level directories from the package into that temporary directory.

Then fpkg chroots into that environment and runs the command matching the package name, or, if that is not a available, panics. There are some directories which will be bind mounted from the host filesystem instead of from a package's environment. The directories are

- `/home`: Users files
- `/dev`: Device files
- `/mnt`: Useful for reading external medium
- `/media`: Same as `/mnt`
- `/run` Runtime files
- `/var`: Variable data.
- `/tmp`: Temporary files

Any conflicts of these directories with the directories from the package, the package's directories will be given priority. The runtime directory is located at `/fpkg/run` by default, but this can be changed by placing the name of a directory in `/etc/fpkg/run`.

_Example_

```
-> = bind mount

/fpkg/run/ad3GH4
├── fpkg
│   ├── env -> /fpkg/env
│   └── pool -> /fpkg/pool
├── fpkg-root -> /
├── usr -> /fpkg/env/my-pkg-3.5.11/usr
├── bin -> /fpkg/env/my-pkg-3.5.11/bin
├── ... (Package files)
├── home -> /fpkg-root/home
├── dev -> /fpkg-root/dev
└── ... (System files)
```
