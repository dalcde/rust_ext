from ext import *

m = module.FDModuleBuilder(2).add_generator(0, "x").build()
r = m.resolve()
r.resolve_through_degree(20)
print(r.graded_dimension_string())
