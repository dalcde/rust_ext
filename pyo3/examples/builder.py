from ext import *

module = FDModuleBuilder(3)\
        .set_name("C(3, alpha_1)")\
        .add_generator(0, "x0")\
        .add_generator(1, "x1")\
        .add_action("b x0 = x1")\
        .add_generator(4, "x4")\
        .add_generator(5, "x5")\
        .add_action("b x4 = x5")\
        .add_action("P1 x0 = x4")\
        .add_action("P1 x1 = x5")\
        .build()

print(module.to_json())
