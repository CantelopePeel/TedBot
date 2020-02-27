import sys
sys.path.append('./BERT-SQuAD')
import json
import torch
from bert import QA

print("Loading model!", file=sys.stderr, flush=True)
model = QA('model')
print("Model loading complete!", file=sys.stderr, flush=True)


def predict(doc, q):
    answer = model.predict(doc, q)
    content = json.dumps(answer, separators=(',', ':'))
    print(content, flush=True)
