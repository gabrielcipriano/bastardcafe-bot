
# %%
import requests
import xml.etree.ElementTree as ET
import time
import random
from bs4 import BeautifulSoup
from dotenv import load_dotenv

import os
import json

load_dotenv()
games_list_url = os.getenv("GAMES_LIST_URL")

# %%
def get_game_urls():
    game_urls = []
    page = 1
    while True:
      result = requests.get(f"{games_list_url}{page}.xml")

      try:
        root = ET.fromstring(result.text)
      except ET.ParseError:
          print(f"Got error on page {page}. probably no more pages.")
          break

      print(f"Got page {page} with {len(root)} urls")
      for url_node in root:
          for locale_node in url_node:
              game_urls.append(locale_node.text)
      page += 1
    return game_urls
# %%

game_urls = get_game_urls()
print(f"Got {len(game_urls)} urls")
# %%
# split game urls in 3 chunks

game_urls_chunks = [game_urls[i:i + len(game_urls)//3+1] for i in range(0, len(game_urls), len(game_urls)//3+1)]
print("chunks:", [len(chunk) for chunk in game_urls_chunks])


# %%
def process_chunk(urls, pid):
  games_info = {}
  count = 0
  fails = []

  # a print function with process id
  printp = lambda x: print(f"[{pid}] {count:4} {x}")

  try:
    with open(f'games_{pid}.json', 'r') as f:
      games_info = json.load(f)
      printp(f"Loaded {len(games_info)} games")
  except FileNotFoundError:
    printp("No file found. Starting from scratch")

  for game_url in urls:
    count += 1
    if game_url in games_info:
      if count % 50 == 0:
        printp(f"⏩️ Skipping {game_url}")
      continue
    if count % 50 == 0:
      printp(f"⏱️ taking a break. Processed {count} games. Failed on {len(fails)} games.")
      time.sleep(1)
    try: 
      result = requests.get(game_url, timeout=2)
      if result.status_code != 200:
          raise Exception(f"Failed to get {game_url}. Status code: {result.status_code}")
      soup = BeautifulSoup(result.text, 'html.parser')

      # <th>Location</th>
      locations_table = soup.find('th', string='Location').find_next_sibling('td').find('dl').find_all('dt')

      locations = []
      for location in locations_table:
          locations.append({
              "store": location.text,
              "locale": location.find_next_sibling('dd').text
          })

      game_name = soup.find('article').find('h1').text.strip()

      games_info[game_url] = {
          "url": game_url,
          "name": game_name,
          "locations": locations,
          "raw": str(soup.find('article'))
      }
      printp(f"✅ Processed {game_name}")
    except requests.exceptions.Timeout as e:
      fails.append([game_url, {"error": str(e)}])
      printp(f"🟡 Timeout on {game_url.split("/")[-2]}. Retrying later")
    except Exception as e:
      fails.append([game_url, {"error": str(e)}])
      printp(f"🔴 Failed on {game_url.split("/")[-2]}")
      printp(e)
  printp(f"Processed a total of {count} games")
  printp(f"Failed on {len(fails)} games")
  with open(f'games_{pid}.json', 'w') as f:
    json.dump(games_info, f)
  with open(f'fails_{pid}.json', 'w') as f:
    json.dump(fails, f)
  
# %%

process_chunk(game_urls_chunks[0], 0)
# %%
# multiprocess
from threading import Thread

threads = []

for i in range(3):
  t = Thread(target=process_chunk, args=(game_urls_chunks[i], i))
  t.start()
  threads.append(t)

for t in threads:
   t.join()
   print("Thread finished...exiting")
print("All threads finished")
# %%
#join jsons
games_info = {}

for i in range(len(game_urls_chunks)):
  with open(f'games_{i}.json', 'r') as f:
    games_info.update(json.load(f))

print(f"Total games: {len(games_info)}")
with open('games_info_06_29.json', 'w') as f:
    json.dump(list(games_info.values()), f)
# %%
