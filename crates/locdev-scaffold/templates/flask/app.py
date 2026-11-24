# Run by LocDev with `flask --app app run --host 127.0.0.1 --port <port>`.
from flask import Flask

app = Flask(__name__)


@app.get("/")
def index():
    return "<h1>It works</h1><p>Flask, served by LocDev.</p>"
