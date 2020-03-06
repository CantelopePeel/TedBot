import requests
import re
import time
from bs4 import BeautifulSoup, NavigableString
from urllib.parse import  urljoin
from collections import deque
import json
import html2text

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
        if a_tag.get('href') is not None:
            page_link = urljoin(url, a_tag.get('href')).split("#")[0]
            #print(page_link)
            page_link = filter_or_fix_link(page_link, parent_link)
            #print(page_link)
            if page_link is not None:
                link_queue.append(page_link)

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
    
    for p in include_patterns:
        if p.match(page_link):
            break
    else: 
        return None
            
    if not page_link.startswith("https://"):
        page_link = "https://" + page_link
    if page_link in link_set or page_link in link_queue:
        return None

    return page_link


def download_page(url):
    res = requests.get(url, timeout=10.0)
    #print(dir(res),res.status_code)
    if "text/html" not in res.headers['Content-Type']:
        return None
    if res.status_code != 200:
        return None
    return res

def delete_useless_tags(html_page):
    soup = BeautifulSoup(html_page, 'html.parser')
    useless_tags = ["footer", "nav"]
    for useless_tag in useless_tags:
        for page_tag in soup.find_all(useless_tag):
            page_tag.extract()
    for a_tag in soup.find_all('a'):
        if a_tag.get('href') is not None:
            if a_tag.get('href').startswith("#"):
                a_tag.extract()
    for div_tag in soup.select('ul[class*="menu"]'):
        div_tag.extract()
    return str(soup)


def finalize_document(titles, bodies, url):
    doc = {
            "title": list(filter(lambda t: t is not None, titles)),
            "body": bodies,
            "link": [url],
            }
    return doc


def parse_md_to_document(md_page, url):
    titles = [None for _ in range(5)]
    bodies = []
    documents = []
    current_body = ""
    for line in md_page.splitlines(): 
        if line.startswith("#"):
            if len(bodies) != 0:
                documents.append(finalize_document(titles, bodies, url))
            bodies = []
            heading_type, heading_text = line.split(" ", 1)
            heading_num = heading_type.count("#") - 1
            for i in range(heading_num, 5):
                titles[i] = None
            titles[heading_num] = heading_text
        elif line == "* * *":
            if len(bodies) != 0:
                documents.append(finalize_document(titles, bodies, url))
            bodies = []
        elif line == "" and current_body != "":
            bodies.append(current_body)
            current_body = ""
        else:
            if current_body != "":
                current_body += "\n"
            current_body += line
    if len(bodies) != 0:
        documents.append(finalize_document(titles, bodies, url))
    return documents

def process_page(url):
    global link_counter

    if url in link_set:  
        return ""
    print(link_counter,  url in link_set,  len(link_queue), url)
    link_counter += 1

    res = download_page(url)
    if res is None:
        return None
    html_page = res.content
    extract_links(html_page, url)

    html_page = delete_useless_tags(html_page)
    md_page = convert_html_to_md(str(html_page))
    docs = parse_md_to_document(md_page, url)
    
    return docs

def convert_html_to_md(html_page):
    page_converter = html2text.HTML2Text()

    page_converter.ignore_emphasis = True
    page_converter.pad_tables = True 
    page_converter.ignore_links = True
    page_converter.ignore_images = True
    page_converter.dash_unordered_list = True
    page_converter.body_width = 0
    page_converter.unicode_snob = True
    page_converter.ul_item_mark = "-"
    return page_converter.handle(html_page)

domains = ['www.cs.rochester.edu']
exclude_patterns = [
        re.compile(r".*/research/.*"), 
        re.compile(r".*/seminar/.*"), 
        re.compile(r".*mail.*"), 
        re.compile(r".*profile.*"),
        re.compile(r".*[~].*"),
        re.compile(r"\.pdf$"),
        re.compile(r"\.mov$"),
        re.compile(r"^ftp:"),
        re.compile(r"^mailto:"),
        ]
include_patterns = [
        re.compile(r".*rochester.edu$"),
        re.compile(r".*/graduate/.*"), 
        re.compile(r".*/undergraduate/.*"), 
        ]


with open('json_content/content.txt', 'w') as content_file:
    pass

counter = 0
link_queue.extend(domains)
while len(link_queue) != 0: 
    url = filter_or_fix_link(link_queue.popleft(), "")
    if url is None:
        continue
    if url in link_set:
        continue
   
    try:
        content = process_page(url)
    except IndexError:
        continue
    
    if content is not None:
        with open('json_content/content.txt', 'a') as content_file:
            for doc in content:
                content_file.write(json.dumps(doc, separators=(',', ':')) + "\n")
                content_file.flush()
    link_set.add(url)
    time.sleep(0.1)
    counter += 1

#with open('content.txt', 'w') as content_file:
#    for url in urls:
#        content = extract_text(url)
#        content_file.write(content)


