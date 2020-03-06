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
 - Investigate using inheritance to avoid manually converting FiniteModule to
   AnyModule
 - Add "dual module" construction so that we can construct Hom modules
 - Improve FDModuleBuilder and implement FPModuleBuilder
    - Support custom profiles

 - Avoid panics in general. This causes the whole python repl to terminate.
   Instead, raise Exceptions which are handled gracefully.
 - Produce a package instead of module so that we can
```python
from ext.module import *
```
