from ext import *

c2 = FiniteModule("C2.json").as_anymodule()
ceta = FintieModule("Ceta.json").as_anymodule()

# We need to specify the maximum non-zero degree (it is okay to overestimate;
# the module will be truncated if this is underestimated)
y = c2.tensor(ceta).as_finite_module(3)
print(y.to_json())
