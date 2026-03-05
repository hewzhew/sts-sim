import urllib.request
import json
req = urllib.request.Request('https://api.github.com/repos/ForgottenArbiter/CommunicationMod/issues?state=all', headers={'User-Agent': 'Mozilla/5.0'})
try:
    resp = urllib.request.urlopen(req)
    issues = json.loads(resp.read().decode('utf-8'))
    for i in issues:
        print(f"#{i['number']}: {i['title']} (State: {i['state']})")
except Exception as e:
    print(e)
