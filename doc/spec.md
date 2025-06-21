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

### Glue

Dpt glues are small wrappers that fulfill some requirement of a given package. Available glues are

- `Bin`: Creates small wrappers in the `/usr/bin` directory for all other packages in the dptfile.

- `Glob([globs])` Creates hard-links from all files matching this glob in other packages into the current environment.

To specify glues for a package, specify your intended glue and arguments on a `glue` node. e.g.

```ron
glue: [
    Bin,
    Glob(
        [
            "/usr/lib/systemd/system/*"
        ]
    )
]
```

# Packages

This section defines various properties of packages, as well as their creation.

## Directory Structure

The directory structure of an dpt is quite basic, consisting of an FHS compliant directory along with some dpt metadata. An example package is given below:

```
example-1.2.3
├── dpt
│    ├── .done # Signifies that the package was fully install. DON'T include this when you create a dpt file! This is created when the package is installed.
│    └── pkg.ron # Package details. You write this.
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

```ron
# pkg-directory/dpt/pkg.ron
(
    name: "example",
    version: "1.2.3"

    depends: [
        (
            name: "python",
            version: ">=3.12"
        ),
        (
            name: "coreutils",
            version: "",
        ),
        (
            name: "love",
            version: ""
        ),
        (
            name: "joy",
            version: ""
        )
    ]
)
...
```

Version ranges are specified immediately prior to the version. They can be one of the following

- `>`
- `>=`
- No prefix requires the exact version specified

### DPTBUILDs

For convenience in the process of generating packages, one can write an DPTBUILD file, which is very similar to Arch Linux's PKGBUILDs. Not all features are supported. The currently defined variables/functions in DPTBUILDs are

- `pkgname`: The name of the package.
- `pkgver`: The version of the package.
- `depends`: The dependencies.
- `makedepends`: The build dependencies.
- `build()`: The function that runs the build.
- `glue_bin`: If defined, the `Bin` glue will be specified.
- `glue_glob`: If defined, each item in this list will be an entry for the `Glob` glue.

The build will happen in an dpt environment with only the packages specified in the `makedepends` variable, `bash` and `coreutils`.

If `pkgname` is an array, then all of the `build_${pkgname_item}`s will be called with a unique `pkgdir` but the same source directory. e.g. If `pkgname=( test 'test-libs')` then `test_build` and `test-libs_build` will be called in order and packaged individually. If `pkgname` is specified this way, then for each package one can override `pkgver` and `depends` by prefixing them with the package name and replacing `-` with `_`. e.g. `depends` becomes `test_libs_depends`.

## Repository Format

Repositories are simply http(s) servers with a predefined file structure as follows:

- index.ron: KDL file with a list of packages and package versions that are contained in this repository.

- \*.dpt: All of the compressed dpts on this repository.

index.kdl is composed of the list `packages`. Each element in this list included a `name`, `version`, `url`, and `depends`. e.g.

```ron
(
    packages: [
        (
            name: "python",
            version: "3.10.2",
            url: "/python-3.10.2.dpt",
            depends: [
                (
                    name: "glibc",
                    version: ""
                )
            ]
        )
    ]
)
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

_Example_

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

The dpt system configuration file is located at `${dpt_directory}/dpt.ron`. All generated files from this configuration will be added to the `${dpt_directory}/base` directory. When `dpt rebuild` is run, an `dpt.lock` file is created in the same directory, containing computed information that was derived from `dpt.ron`. This lock file includes generated information such as package versions, enabled services, `base` files, etc. `${dpt_directory}/dpt.ron` has the following fields:

- `packages` An array of packages. If the version is left blank, the newest version will be used.

- `users` A list of users on the system. This array will be used to auto-generate the `/etc/passwd` file. The required fields are `username`, `password`, `uid`, `gid`, `gecos`, `home` and `shell`.

- `groups` A list of groups on the system, and their members. This array will be used to auto-generate `/etc/group`. The required fields are `groupname`, `gid`, and `members`. Note that `members` is a list of strings.

- `services` A map of targets with a list of enabled services. e.g.
  ```ron
  services: {
      "multi-user.target.wants": [
          "stuff.service",
          "sock.socket"
      ]
  }
  ```
