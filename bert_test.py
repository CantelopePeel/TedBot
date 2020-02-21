import sys
sys.path.append('./BERT-SQuAD')
from bert import QA

model = QA('model')

def predict(doc, q):
    answer = model.predict(doc, q)
    return answer

print(predict(sys.argv[1], sys.argv[2]))
