import time
import json
import urllib.request
import os

transcript_path = "/Users/sam.gl/.gemini/antigravity/brain/dcaad95a-be83-4785-9636-bf935bf3676b/.system_generated/logs/transcript.jsonl"
url = "http://127.0.0.1:9528/api/v1/notify"

def post_json(payload):
    try:
        req = urllib.request.Request(url, data=json.dumps(payload).encode('utf-8'), headers={'Content-Type': 'application/json'})
        urllib.request.urlopen(req)
    except Exception as e:
        print(f"HTTP Error: {e}")

def process_line(line):
    try:
        data = json.loads(line)
        step_type = data.get("type")
        
        # When agent is thinking/using tools
        if step_type == "PLANNER_RESPONSE" and "tool_calls" in data:
            for tc in data["tool_calls"]:
                payload = {
                    "source_client": "antigravity",
                    "hook_event_name": "PreToolUse",
                    "payload": {
                        "toolCall": {
                            "name": tc["name"],
                            "args": tc["args"]
                        }
                    }
                }
                post_json(payload)
        
        # When user inputs, session effectively resumes/starts
        elif step_type == "USER_INPUT":
            payload = {
                "source_client": "antigravity",
                "hook_event_name": "SessionStart",
                "payload": {}
            }
            post_json(payload)
            
        # When a tool finishes
        elif step_type in ["RUN_COMMAND", "VIEW_FILE", "REPLACE_FILE_CONTENT", "MULTI_REPLACE_FILE_CONTENT", "READ_URL_CONTENT"]:
            payload = {
                "source_client": "antigravity",
                "hook_event_name": "PostToolUse",
                "payload": {}
            }
            post_json(payload)

    except Exception as e:
        print(f"Error parsing line: {e}")

print(f"Starting Antigravity Hook Injector... Tailing {transcript_path}")
with open(transcript_path, 'r') as f:
    # Seek to end so we only get new events
    f.seek(0, 2)
    while True:
        line = f.readline()
        if not line:
            time.sleep(0.5)
            continue
        process_line(line)
