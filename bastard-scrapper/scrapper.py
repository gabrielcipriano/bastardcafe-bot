
# %%
import requests
import xml.etree.ElementTree as ET
import time
import random
from dotenv import load_dotenv

import os

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

from bs4 import BeautifulSoup

# %%

first_game = "https://bastardcafe.dk/games/7-wonders-duel/"

soup = BeautifulSoup(requests.get(first_game).text, 'html.parser')

# <th>Location</th>
locations_table = soup.find('th', string='Location').find_next_sibling('td').find('dl').find_all('dt')

for location in locations_table:
    print(location.text)
    print(location.find_next_sibling('dd').text)

game_name = soup.find('article').find('h1').text.strip()

print(game_name)
# %%
games_info = {}
fails = []

# %%
count = 0
fails = []
for game_url in game_urls:
  count += 1
  if game_url in games_info and "raw" in games_info[game_url]:
    print(f"Skipping {game_url}")
    continue
  if count % 50 == 0:
    print(f"taking a break. Processed {count} games. Failed on {len(fails)} games.")
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
    print(f"Processed {game_name}")
  except requests.exceptions.Timeout as e:
    fails.append([game_url, e])
    print(f"Timeout on {game_url}. Retrying later")
  except Exception as e:
    fails.append([game_url, e])
    print(f"Failed on {game_url}")
    print(e)
print(f"Processed a total of {count} games")
print(f"Failed on {len(fails)} games")
# %%
print(fails)
# %%
games_info["https://bastardcafe.dk/games/21-days/"]["raw"]
# %%
# SAVE TO JSON
import json

with open('games_info_v2.json', 'w') as f:
    json.dump(games_info, f)

with open('board_games_list_v2.json', 'w') as f:
    json.dump(list(games_info.values()), f)
# %%
# size of the dict
len(games_info)
# %%
fails
# %%
