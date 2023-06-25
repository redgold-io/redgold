Model interop setup with rust for redgold


```

git clone https://github.com/redgold-io/GraphEmbedding.git

#ubuntu
sudo apt-get install -y python3-venv

#mac
pip install virtualenv

python3 -m venv venv

source venv/bin/activate

# pip install -r requirements.txt

#brew install hdf5



python3 -m pip install Cython
python3 -m pip install pkgconfig
python3 -m pip install tensorflow

python3 setup.py install develop


venv/bin/python -m pip install pyinstaller
venv/bin/pyinstaller main.py

```