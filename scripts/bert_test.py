import sys
sys.path.append('./BERT-SQuAD')
import json
import torch
from bert import QA

def predict(doc, q):
    answer = model.predict(doc, q)
    content = json.dumps(answer, separators=(',', ':'))
    print(content, flush=True)

print("Loading model!", flush=True)
model = QA('model')
print("Model loading complete!", flush=True, end='')
model.predict("Whales are a kind of animal called a mammal.", "What is a whale?")

