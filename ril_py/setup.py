from setuptools import setup
from setuptools_rust import Binding, RustExtension

setup(
    name="ril",
    version="0.1.0",
    rust_extensions=[RustExtension("ril.ril", binding=Binding.PyO3)],
    packages=["ril"],
    # rust extensions are not zip safe, just like C-extensions.
    zip_safe=False,
)
