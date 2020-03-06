from ext import *

target = module.FDModuleBuilder(2)\
            .add_generator(0, "x0")\
            .add_generator(1, "x1")\
            .add_action("Sq1 x0 = x1")\
            .build()

source = target.as_anymodule().tensor(target.as_anymodule()).as_finite_module(target.max_degree() * 2)
base = target.element_name(0, 0)

etaL = homomorphism.FDModuleHomomorphismBuilder(source, target, 0);
etaR = homomorphism.FDModuleHomomorphismBuilder(source, target, 0);

for t in range(target.min_degree(), target.max_degree() + 1):
    for n in range(0, target.dimension(t)):
        name = target.element_name(t, n)
        etaL.set("{}.{}".format(name, base), name)
        etaR.set("{}.{}".format(base, name), name)

source_r = source.resolve()
target_r = target.resolve()

etaL = etaL.build().lift(source_r, target_r)
etaR = etaR.build().lift(source_r, target_r)
