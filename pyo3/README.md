# Build instructions
## Setup
```console
 $ pip install --user maturin wheel
 $ rustup install nightly
 $ rustup override set nightly
```
## Build
```console
 $ maturin build
 $ pip install --user --force-reinstall target/wheels/ext-....whl
```

## Run
See `examples/` (just run the python script as usual after the previous commands)

# Plans
In the short term, a good goal would be to make it easy to use this to
mainpulate modules so that we can construct tensor products etc. easily and
save the resulting file.

 - Investigate using inheritance to avoid manually converting FiniteModule to
   AnyModule
 - Add "dual module" construction so that we can construct Hom modules
 - Improve FDModuleBuilder and implement FPModuleBuilder
    - Support custom profiles

Next we should expose resolutions and resolution homomorphisms, so that we can
use this to compute maps between Ext.
