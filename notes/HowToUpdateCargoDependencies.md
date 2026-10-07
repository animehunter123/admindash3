# To check if the cargo.toml has the latest dependencies and test against them

See below,

```bash

cargo install cargo-update  
cargo update -v   

cargo install cargo-upgrades 
root@ubuntuserver01 ~/d/a/app (main) [101]# mv /tmp/cargo-upgrade ~/.cargo/bin                                                               (base)
root@ubuntuserver01 ~/d/a/app (main)# cargo-upgrade                                                                                          (base)
Usage: cargo <COMMAND>

Commands:
  upgrade  Upgrade dependency version requirements in Cargo.toml manifest files
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help

root@ubuntuserver01 ~/d/a/app (main) [2]# cargo upgrade -v                                                                                   (base)
    Checking admindash3's dependencies
note: Re-run with `--verbose` to show more dependencies
  latest: 5 packages
root@ubuntuserver01 ~/d/a/app (main)#                                        


cargo install cargo-machete  

cargo machete
```
