# Introduction

This document outlines the fpkg method of distributing software. Fpkg is centered around the points of

- Reproducibility: I should be able to give my package to a friend and it should work exactly the same as it did on my system.

- Stability: I should be able to install many combinations of packages and not have anything break. Further, if something does break, I should be able to easily roll back.

Reproducibility could be defined as consistent outcomes. When a software developer releases a package, they want it to work on all system configurations. Conventional tools fail on this point as the developer have no idea what configuration their users may have on their system. fpkg excels in this area, however, as all applications have a well defined environment that the developer controls, unaffected by any other packages the user has installed.

Stability is another important topic. Any package manager should be stable, and never require breaking another package to install another. Dependency hell is an unfortunate result from conventional solutions, but efforts to resolve this have been few and far between.

There are a bunch of terms and ideas used in this document.

- fpkg directory: The fpkg directory is located by default at `/fpkg` but the location can be customized by placing a directory name in the file `/etc/fpkg/dir`

- fpkg store: The directory containing all of the packages installed in the system, located at `${fpkg_directory}/store`.

- Package: A single package, often located in an fpkg store.

- Repository: A location on the internet or locally that provides packages to fpkg.

- Dash args: A KDL convention where child nodes named `-` are treated as arrayish. e.g.
  
  ```kdl
  foo {
    - 1
    - 2
    - #false
  }
  ```

# Command line usage

Covers the basics of fpkg’s command line usage. Do note that fpkg should be installed SUID as running packages requires the `chroot` syscall.

## fpkg rebuild

Rebuild the system according to the file fpkg system configuration file. Will also update the system if the repositories are available.

### fpkg run \[package\] \[args\]

Runs the package specified. All other arguments will be passed to the package.

### fpkg run-multi \[packages\] -- \[args\]

Runs the first package specified in an environment that also includes the others.

### fpkg gen-pkg

Generates a package from a directory.

# Inner details

Covers the inner and implementation details of fpkg.

## The fpkg store

The fpkg store are composed of many directories with names following the pattern package-name-1.2.3. The main store is located at `${fpkg_directory}/store`.

_Example fpkg store_

```
/fpkg/store
├── example-1.2.3
│    └── ...
│
├── random-lib-4.5.6
│    └── ...
└── ...
```

## Package environments

For each package, when it is ran, an environment is created. Each environment consists of hardlinks to the main files inside the package and it’s dependencies. Each packages environment will also include files specified in the `${fpkg_directory}/base` directory. If `${fpkg_directory}/base` does not exist or is not a directory then fpkg will just give a warning.

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

The process of generating packages requires writing a description file, installing the program to a directory, and then running `fpkg gen-pkg [directory]`. An example package description file:

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

For convenience in the process of generating packages, one can write an FPKGUILD file, which is very similar to Arch Linux's PKGBUILDs. Not all features are supported. The currently defined variables/functions in FPKGBUILDs are

- `pkgname`
- `pkgver`
- `depends`
- `makedepends`
- `build()`

The build will happen in an fpkg environment with only the packages specified in the `makedepends` variable, `bash` and `coreutils`.

## Repository Format

Repositories are simply http(s) servers with a predefined file structure as follows:

- index.kdl: KDL file with a list of packages and package versions that are contained in this repository.

- *.fpkg: All of the compressed fpkgs on this repository.

index.kdl is made of bunch of package nodes. In each node there is a name value, a version value, and a path value. E.g.

```
package name=python version="3.10.2" path="/python-3.10.2.fpkg" {
depends ...
depends ...
... // A copy of the package's depends section
}
```

The list of repositories is stored in `${fpkg_directory}/repos` in the format of

```
https://pkg.repo/fpkg
https://another.repo
```

The repository's priorities decrease down the file i.e. The first repository has more priority then the second, and the second has more priority then the third etc.

# Dependency resolving

For dependency resolving, fpkg uses [PubGrub](https://crates.io/crates/pubgrub) due to it’s efficient and accurate dependency resolution.

# Package running

When running a package, fpkg will bind `/home`, `/dev`, `/mnt`, `/media`, `/run`, `/var`, `/tmp`, `${fpkg_directory}` inside the environment. If any conflicts with the aforementioned directories and the directories from the package(s) occur, the package's directories will be given priority. The runtime directory is located at `${fpkg_directory}/run`, which is where the environment will be created.

*Example*

```
-> = bind mount

/fpkg/run/ad3GH4
├── ${fpkg_directory} -> ${fpkg_directory}
├── usr
├── ... (Package files)
├── home -> /home
├── dev -> /dev
└── ... (Higher level files)
```

# Fpkg system configuration

The fpkg system configuration file is located at `${fpkg_directory}/fpkg.kdl` and is composed of a key-value KDL document. When `fpkg rebuild` is run, an `fpkg.lock` file is created in the same directory, containing computed information that was computed from `fpkg.kdl`. This lock file includes information such as package versions, enabled services, `base` files, etc. `${fpkg_directory}/fpkg.kdl` has the following fields:

- `packages` A dash array of packages.
