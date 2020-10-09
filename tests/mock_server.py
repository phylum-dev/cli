import time
from uuid import uuid4
from flask import Flask, json, request
from voluptuous import Schema, Required, All, Length

from static_asserts import AssertsAccessor as AA

api = Flask(__name__)
api.config['ENV'] = 'development'
api.config['DEBUG'] = True
api.config['TESTING'] = True

jobs = dict()
job_status = dict()


def build_job(job_id):
    started_at = int(time.time())
    status_it = update_status()
    job_status[job_id] = status_it
    return {
            'id': '59482a54-423b-448d-8325-f171c9dc336b',
            'last_updated': started_at,
            'started_at': started_at,
            'status': 'PENDING',
            'user_id': '86bb664a-5331-489b-8901-f052f155ec79',
            'packages':
            [{
                'heuristics': [{'score': 3.14, 'data': {'foo': 'bar'}}],
                'last_updated': started_at,
                'license': None,
                'name': 'foo',
                'risk': 60,
                'status': 'NEW',
                'type': 'npm',
                'version': '1.0.0',
                'vulnerabilities': [],
                'dependencies':
                [
                    {
                        'heuristics': [],
                        'last_updated': started_at,
                        'license': None,
                        'name': 'bar',
                        'risk': 60,
                        'status': 'COMPLETED',
                        'type': 'npm',
                        'version': '2.3.4',
                        'vulnerabilities': []
                    },
                    {
                        'heuristics': [{'score': 42, 'data': None}],
                        'last_updated': started_at,
                        'license': None,
                        'name': 'baz',
                        'risk': 60,
                        'status': 'NEW',
                        'type': 'npm',
                        'version': '9.8.7',
                        'vulnerabilities': []
                    }
                ],
           }]
        }

def update_status():
    for status in ('NEW', 'PENDING', 'COMPLETED'):
        yield status

def validate_request(req_json, schema):
    print(req_json)
    schema(req_json)

@api.route('/request/packages', methods=['PUT'])
def put_packages():
    schema = Schema({
        'packages': [{
            Required('name'): str,
            Required('version'): str,
            Required('type'): str,
        }],
    })
    validate_request(request.json, schema)

    job_id = str(uuid4())
    jobs[job_id] = build_job(job_id)
    print(f"Added job to db: {jobs[job_id]}")
    return json.dumps({'job_id': job_id}), 201

@api.route('/request/packages/<job_id>', methods=['GET'])
def get_status(job_id):
    job = jobs.get(job_id, {})
    if job:
        status = job_status.get(job_id, {})
        job['last_updated'] = int(time.time())
        job['status'] = next(status)
        code = 200
    else:
        code = 404

    return json.dumps(job), code

@api.route('/auth/login', methods=['POST'])
def auth_login():
    schema = Schema({
        Required('login'): str,
        Required('password'): str,
    })
    validate_request(request.json, schema)

    return {
        'access_token'  : 'asdfghjkl',
        'refresh_token' : 'qwertyuio',
    }

if __name__ == '__main__':
    api.run(debug=True)
