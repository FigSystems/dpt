# Introduction

This document outlines the dpt method of distributing software. Dpt is centered around the points of

- Reproducibility: I should be able to give my package to a friend and it should work exactly the same as it did on my system.

- Stability: I should be able to install many combinations of packages and not have anything break. Further, if something does break, I should be able to easily roll back.

Reproducibility could be defined as consistent outcomes. When a software developer releases a package, they want it to work on all system configurations. Conventional tools fail on this point as the developer have no idea what configuration their users may have on their system. dpt excels in this area, however, as all applications have a well defined environment that the developer controls, unaffected by any other packages the user has installed.

Stability is another important topic. Any package manager should be stable, and never require breaking another package to install another. Dependency hell is an unfortunate result from conventional solutions, but efforts to resolve this have been few and far between.

There are a bunch of terms and ideas used in this document.

- dpt directory: The dpt directory is located by default at `/dpt` but the location can be customized by placing a directory name in the file `/etc/dpt/dir`

- dpt store: The directory containing all of the packages installed in the system, located at `${dpt_directory}/store`.

- Package: A single package, often located in an dpt store.

- Repository: A location on the internet or locally that provides packages to dpt.

- Dash args: A KDL convention where child nodes named `-` are treated as arrayish. e.g.
  
  ```kdl
  foo {
    - 1
    - 2
    - #false
  }
  ```

# Command line usage

Covers the basics of dpt’s command line usage. Do note that dpt should be installed SUID as running packages requires the `chroot` syscall.

## dpt rebuild

Rebuild the system according to the file dpt system configuration file. Will also update the system if the repositories are available.

## dpt run \[package\] \[args\]

Runs the package specified. All other arguments will be passed to the package.

## dpt run-multi \[packages\] -- \[args\]

Runs the first package specified in an environment that also includes the others.

## dpt dev-env [packages] -- [args]

Fetches the packages if they are not found into the store, and runs them in the same ways as run-multi does. Only intended for the purpose of `makedpt` and other development related tasks.

## dpt gen-pkg

Generates a package from a directory.

# Inner details

Covers the inner and implementation details of dpt.

## The dpt store

The dpt store are composed of many directories with names following the pattern package-name-1.2.3. The main store is located at `${dpt_directory}/store`.

_Example dpt store_

```
/dpt/store
├── example-1.2.3
│    └── ...
│
├── random-lib-4.5.6
│    └── ...
└── ...
```

## Package environments

For each package, when it is ran, an environment is created. Each environment consists of hardlinks to the main files inside the package and it’s dependencies. Each packages environment will also include files specified in the `${dpt_directory}/base` directory. If `${dpt_directory}/base` does not exist or is not a directory then dpt will just give a warning.

# Packages

This section defines various properties of packages, as well as their creation.

## Directory Structure

The directory structure of an dpt is quite basic, consisting of an FHS compliant directory along with some dpt metadata. An example package is given below:

```
example-1.2.3
├── dpt
│    └── pkg.kdl # Package details. You write this.
└── usr
    ├── bin
    │    └── example
    └── share
         └── example
              └── example.txt
```

Most dpts are distributed as `.dpt` files. A `.dpt` file is just a zstd compressed tar archive containing the dpt.

## Generating packages

The process of generating packages requires writing a description file, installing the program to a directory, and then running `dpt gen-pkg [directory]`. An example package description file:

```
# pkg-directory/dpt/pkg.kdl
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

### DPTBUILDs

For convenience in the process of generating packages, one can write an DPTUILD file, which is very similar to Arch Linux's PKGBUILDs. Not all features are supported. The currently defined variables/functions in DPTBUILDs are

- `pkgname`
- `pkgver`
- `depends`
- `makedepends`
- `build()`

The build will happen in an dpt environment with only the packages specified in the `makedepends` variable, `bash` and `coreutils`.

## Repository Format

Repositories are simply http(s) servers with a predefined file structure as follows:

- index.kdl: KDL file with a list of packages and package versions that are contained in this repository.

- *.dpt: All of the compressed dpts on this repository.

index.kdl is made of bunch of package nodes. In each node there is a name value, a version value, and a path value. E.g.

```
package name=python version="3.10.2" path="/python-3.10.2.dpt" {
depends ...
depends ...
... // A copy of the package's depends section
}
```

The list of repositories is stored in `${dpt_directory}/repos` in the format of

```
https://pkg.repo/dpt
https://another.repo
```

The repository's priorities decrease down the file i.e. The first repository has more priority then the second, and the second has more priority then the third etc.

# Dependency resolving

For dependency resolving, dpt uses [PubGrub](https://crates.io/crates/pubgrub) due to it’s efficient and accurate dependency resolution.

# Package running

When running a package, dpt will bind `/home`, `/dev`, `/mnt`, `/media`, `/run`, `/var`, `/tmp`, `${dpt_directory}` inside the environment. If any conflicts with the aforementioned directories and the directories from the package(s) occur, the package's directories will be given priority. The runtime directory is located at `${dpt_directory}/run`, which is where the environment will be created.

*Example*

```
-> = bind mount

/dpt/run/ad3GH4
├── ${dpt_directory} -> ${dpt_directory}
├── usr
├── ... (Package files)
├── home -> /home
├── dev -> /dev
└── ... (Higher level files)
```

# Dpt system configuration

The dpt system configuration file is located at `${dpt_directory}/dpt.kdl` and is composed of a key-value KDL document. All generated files from this configuration will be added to the `${dpt_directory}/base` directory. When `dpt rebuild` is run, an `dpt.lock` file is created in the same directory, containing computed information that was computed from `dpt.kdl`. This lock file includes generated information such as package versions, enabled services, `base` files, etc. `${dpt_directory}/dpt.kdl` has the following fields:

- `packages` An array of packages. Each child's node name is the package name and the next argument, if it exists, will be the version.

- `users` A list of users on the system. This array will be used to auto-generate the `/etc/passwd` file. The entries (sub nodes) are in the format of
  
  ```kdl
  username \
      "Hashed password" \
      uid \
      gid \
      "Full Name (GECOS)" \
      "/home/directory" \
      "/usr/bin/my-fave-shell"
  ```

- `groups` A list of groups on the system, and their members. This array will be used to auto-generate `/etc/group`. The entries (sub nodes) are in the format of
  
  ```kdl
  groupname gid {
      member1
      member2
      ...
  }
  ```

- 
