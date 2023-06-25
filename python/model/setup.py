import setuptools


with open("README.md", "r") as fh:

    long_description = fh.read()


REQUIRED_PACKAGES = [
    'ge'
]


setuptools.setup(

    name="ge",

    version="0.0.0",

    author="Redgold",

    author_email="info@redgold.io",

    url="https://github.com/redgold-io/redgold",

    packages=setuptools.find_packages(exclude=[]),

    python_requires='>=3.5',  # 3.4.6

    install_requires=REQUIRED_PACKAGES,
    #
    # extras_require={
    #
    #     "cpu": ['tensorflow>=1.4.0,!=1.7.*,!=1.8.*'],
    #
    #     "gpu": ['tensorflow-gpu>=1.4.0,!=1.7.*,!=1.8.*'],
    #
    # },

    entry_points={

    },
    license="MIT license",


)
