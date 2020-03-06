# TedBot

Run the following commands to set up the workspace.

```
git clone https://github.com/kamalkraj/BERT-SQuAD.git
wget https://www.dropbox.com/s/8jnulb2l4v7ikir/model.zip
mkdir model
unzip -d model model
```

Then you need to set up the environment. We've made a script to initialize this for you! Simply run `start_env.sh`.
This shoudl only need to be run once. 

To make sure your model is able to load correctly, run `./venv/bin/python scripts/bert_test.py` to completion. 

# Run 

Run `cargo run --release ./json_content` and once you feel the time is right, ask it an advising question. 
