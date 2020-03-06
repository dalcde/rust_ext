from ext import *

source = module.FDModuleBuilder(2)\
            .add_generator(0, "x0")\
            .add_generator(1, "x1")\
            .add_action("Sq1 x0 = x1")\
            .build()

target = module.FDModuleBuilder(2)\
            .add_generator(0, "x0")\
            .build()

hom = homomorphism.FDModuleHomomorphismBuilder(source, target, 0)\
            .set("x0", "x0")\
            .build()

source_r = source.resolve()
target_r = target.resolve()

rhom = hom.lift(source_r, target_r)

print("f(1) =", rhom.act(0, 0, 0))
print("f(h_0) =", rhom.act(1, 1, 0))
print("f(h_1) =", rhom.act(1, 2, 0))
