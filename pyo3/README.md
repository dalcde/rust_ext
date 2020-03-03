# Build instructions
## Setup
```console
 $ pip install --user maturin wheel
 $ rustup install nightly
 $ rustup override set nightly
```
## Build
``console
 $ maturin build
 $ pip install --user --force-reinstall target/wheels/ext-....whl

## Run
```python
>>> from ext import *
>>> m = FiniteModule.from_json(r'{"type" : "finite dimensional module", "name": "$C(2)$", "p": 2, "generic": false, "gens": {"x0": 0, "x1": 1}, "actions": ["Sq1 x0 = x1"]}')
>>> (m.dimension(0), m.dimension(1), m.dimension(2))
(1, 1, 0)
>>> n = m.as_anymodule()
>>> npn = n.sum(n)
>>> npn.compute_basis(2)
>>> (npn.dimension(0), npn.dimension(1), npn.dimension(2))
(2, 2, 0)
>>> ntn = n.tensor(n)
>>> (ntn.dimension(0), ntn.dimension(1), ntn.dimension(2))
(1, 2, 1)
>>> ntn = ntn.to_finite_module(2)
>>> ntn.to_json()
'{"actions":["Sq1 x0.x0 = x0.x1 + x1.x0","Sq2 x0.x0 = x1.x1","Sq1 x0.x1 = x1.x1","Sq1 x1.x0 = x1.x1"],"generic":false,"gens":{"x0.x0":0,"x0.x1":1,"x1.x0":1,"x1.x1":2},"name":"","p":2,"type":"finite dimensional module"}'
```

# Plans
In the short term, a good goal would be to make it easy to use this to
mainpulate modules so that we can construct tensor products etc. easily and
save the resulting file.

 - Construct module from file name a la rust_ext::utils::construct
 - Add "dual module" construction so that we can construct Hom modules
 - Produce a ModuleBuilder class in pyo3 bindings that owns the module, so that
   users can start with a trivial module and progressively add actions. There
   should be a way to turn an existing module into a ModuleBuilder object,
   which shall be achieved by cloning. Indeed, if the module is already used
   somewhere else (e.g. in a resolution), modifying the module in place would
   be pretty bad.

Next we should expose resolutions and resolution homomorphisms, so that we can
use this to compute maps between Ext.
