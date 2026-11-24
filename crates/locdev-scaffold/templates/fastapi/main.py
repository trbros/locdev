# Run by LocDev with `fastapi dev main.py --host 127.0.0.1 --port <port>`.
from fastapi import FastAPI

app = FastAPI()


@app.get("/")
def index():
    return {"message": "It works: FastAPI, served by LocDev."}
