# %%
import os
import json
from enum import Enum

from libsql_client import Statement, Client, create_client

from dotenv import load_dotenv


load_dotenv()  # take environment variables from .env.

JSON_FILE = "games_info_06_29.json"

url = os.getenv("LIBSQL_DATABASE_URL")
auth_token = os.getenv("LIBSQL_AUTH_TOKEN")


BATCH_TYPE = "BOARD_GAME_LIST"
# BATCH_TYPE = "TEST"

class BatchStatus(Enum):
    PENDING = "PENDING"
    DONE = "DONE"
    FAILED = "FAILED"
 
async def update_schema():
    async with create_client(url, auth_token=auth_token) as client:
        await client.execute("CREATE TABLE IF NOT EXISTS key_value (id INTEGER PRIMARY KEY, batch_type TEXT, batch_id INTEGER, key TEXT, value JSONB)")
        await client.execute("CREATE INDEX IF NOT EXISTS key_index ON key_value (batch_type, batch_id, key)")
        await client.execute("CREATE TABLE IF NOT EXISTS batch (id INTEGER PRIMARY KEY, batch_type TEXT, status TEXT, details OPTIONAL JSONB, created_at TIMESTAMP, updated_at TIMESTAMP)")
        await client.execute("CREATE INDEX IF NOT EXISTS batch_type_index ON batch (batch_type, id)")
        await client.execute("""CREATE TABLE IF NOT EXISTS bot_metrics ( 
                             id INTEGER PRIMARY KEY, 
                             bot_id TEXT, 
                             chat_id NUMBER, 
                             text TEXT, 
                             ok BOOLEAN, 
                             details JSONB, 
                             response_time_ms NUMBER, 
                             created_at TIMESTAMP
                             )""")


# %%
async def persist_batch(bg_list: list[dict]):
    async with create_client(url, auth_token=auth_token) as client:
        result = await client.execute("INSERT INTO batch (batch_type, status, created_at, updated_at) VALUES (?1, ?2, datetime('now'), datetime('now')) RETURNING id", [
            BATCH_TYPE,
            BatchStatus.PENDING.value
        ])
        batch_id = result.rows[0][0]

        # splitting into chunks
        chunk_size = 100
        bg_list = list(bg_list)
        chunks = [bg_list[i:i + chunk_size] for i in range(0, len(bg_list), chunk_size)]

        print(f"Split into {len(chunks)} chunks")
        for (i, chunk) in enumerate(chunks):
            await send_chunk(client, batch_id, chunk)
            print(f"Sent chunk {i} of {len(chunk)} games")
        
        await client.execute("UPDATE batch SET status = ?1, updated_at = datetime('now') WHERE id = ?2", [BatchStatus.DONE.value, batch_id])


async def send_chunk(client: Client, batch_id: int, chunk: list[dict]):
    statements = []
    for game in chunk:
        statements.append(Statement("INSERT INTO key_value (batch_type, batch_id, key, value) VALUES (?1, ?2, ?3, ?4)", [
            BATCH_TYPE,
            batch_id,
            game["url"],
            json.dumps(game)
        ]))

    await client.batch(statements)


def load_json(file_path):
    with open(file_path) as f:
        return json.load(f)


async def main():
    # updating schema
    await update_schema()

    # loading json
    games_info = load_json(JSON_FILE)
    print(f"Loaded {len(games_info)} games")

    await persist_batch(games_info)

# await update_schema()
await main()

# %%
