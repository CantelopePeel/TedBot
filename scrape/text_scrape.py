import requests
import re
import time
from bs4 import BeautifulSoup, NavigableString
from urllib.parse import  urljoin
from collections import deque
import json

def strip_tags(html, invalid_tags):
    strip_soup = BeautifulSoup(html, 'html.parser')

    for tag in strip_soup.findAll(True):
        if tag.name in invalid_tags:
            s = ""

            for c in tag.contents:
                if not isinstance(c, NavigableString):
                    c = strip_tags(str(c), invalid_tags)
                s += str(c)

            tag.replaceWith(s)

    return strip_soup

link_counter = 0
link_queue = deque()
link_set = set()
read_link_set = set()

def extract_links(resp_content, parent_link):
    link_soup = BeautifulSoup(resp_content, 'html.parser')
    for a_tag in link_soup.find_all('a'):
        page_link = urljoin(url, a_tag.get('href')).split("#")[0]
        #print(page_link)
        page_link = filter_or_fix_link(page_link, parent_link)
        #print(page_link)
        if page_link is not None:
            link_queue.append(page_link)
            with open('new_links.txt', 'a') as links_file:
                links_file.write(page_link + "\n")

def filter_or_fix_link(page_link, parent_link):
    page_link = page_link.replace('http://', 'https://')
    if page_link == '':
        return None
    if page_link[-1] == '/':
        page_link = page_link[:-1]

    if page_link == parent_link:
        return None
    
    for domain in domains:
        if domain in page_link:
            break
    else:
        return None
    
    for p in exclude_patterns:
        if p.match(page_link):
            return None
            
    if page_link in link_set or page_link in link_queue:
        return None

    return page_link

def extract_text(url):
    global link_counter

    if url in link_set:  
        return ""
    print(link_counter,  url in link_set,  len(link_queue), url)
    link_counter += 1
    res = requests.get(url, timeout=10.0)
    #print(dir(res),res.status_code)
    if "text/html" not in res.headers['Content-Type']:
        return ""
    if res.status_code != 200:
        return ""

    extract_links(res.content, url)
    text_allowed_tags = [
            'p',
            'span',
            'li',
    ]
   
    invalid_tags = ['b', 'i', 'u', 'strong', 'a', 'em', 'code', 'img', 'br', 'footer']
    html_page = str(strip_tags(res.content, invalid_tags))

    soup = BeautifulSoup(html_page, 'html.parser')
    
    text = soup.find_all(text=True)

    output = ''

    for t in text: 
        if t.parent.name in text_allowed_tags:
            tag_content = str(t).replace('\n', '').replace('\t', '').replace("  ", " ")
            if len(tag_content) != 0:
                if len(tag_content.split(" ")) >= 10:
                    output += '{}\n\n'.format(tag_content)
    return output


domains = ['www.cs.rochester.edu']
exclude_patterns = [
        re.compile(r".*/research/.*"), 
        re.compile(r".*/seminar/.*"), 
        re.compile(r".*mail.*"), 
        re.compile(r".*[~].*"),
        re.compile(r"\.pdf$"),
        re.compile(r"\.mov$"),
        re.compile(r"^ftp:"),
        re.compile(r"^mailto:"),
        ]

urls = []
with open('urls.txt', 'r') as urls_file:
    for l in urls_file:
        urls.append(l[:-1])

with open('links.txt', 'a') as links_file: 
    pass
with open('new_links.txt', 'a') as links_file: 
    pass

with open('links.txt', 'r') as links_file:
    for l in links_file:
        link_set.add(l[:-1])

with open('new_links.txt', 'r') as links_file:
    for l in links_file:
        if l not in link_set:
            link_queue.append(l[:-1])

with open('urls.txt', 'r') as urls_file:
    for l in urls_file:
        if l not in link_set:
            link_queue.append(l[:-1])

domains = urls
while len(link_queue) != 0: 
    url = link_queue.popleft()
    if url in link_set:
        continue
    with open('links.txt', 'a') as links_file:
        links_file.write(url + "\n")
    
    with open('content.txt', 'a') as content_file:
        content = extract_text(url)
        content_file.write(content)
        content_file.flush()
    link_set.add(url)

    time.sleep(0.1)

#with open('content.txt', 'w') as content_file:
#    for url in urls:
#        content = extract_text(url)
#        content_file.write(content)


