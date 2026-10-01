from setuptools import Extension, setup
setup(name="oyzu-native-example", version="0.1.0", py_modules=["fallback"], ext_modules=[Extension("native_math", ["native_math.c"])])
