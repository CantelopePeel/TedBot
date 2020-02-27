# TedBot

Run the following commands to set up the workspace.

```
git clone https://github.com/kamalkraj/BERT-SQuAD.git
wget https://www.dropbox.com/s/8jnulb2l4v7ikir/model.zip
mkdir model
unzip -d model model
```

Then you need to set up the environment. We've made a script to initialize this for you! Simply run `start_env.sh`.

# Dependencies

Most dependencies are resolved for free! Simply run `rust run ./content` and most stuff will be downloaded for you.
Other dependencies:

- torch
- pytorch-transformers==1.0.0

or for the lazy, install with this (if you're running on cycle, run with `--user` option

```
pip3 install torch pytorch-transformers==1.0.0
```

To make sure your model is able to load correctly, run `python3 scripts/bert_test.py` to completion. 

# Run 

Run `rust run ./content` and once you feel the time is right, ask it an advising question. 
