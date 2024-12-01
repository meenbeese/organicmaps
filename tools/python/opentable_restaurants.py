#!/usr/bin/env python3
# coding: utf-8

import argparse
import base64
import json
import logging
import re
import sys
import requests

# Initialize logging.
logging.basicConfig(level=logging.DEBUG, format='[%(asctime)s] %(levelname)s: %(message)s')


class OpentableDownloaderError(Exception):
    pass


class OpentableDownloader:
    def __init__(self, login, password, opentable_filename, tsv_filename=None):
        self.login = login
        self.password = password
        self.token = None
        self.opentable_filename = opentable_filename
        self.tsv_filename = tsv_filename
        self._get_token()

    def download(self):
        headers = self._add_auth_header({'Content-Type': 'application/json'})
        url = 'https://platform.opentable.com/sync/listings'

        with open(self.opentable_filename, 'w', encoding='utf-8') as f:
            offset = 0
            while True:
                request_url = f"{url}?offset={offset}"
                logging.debug(f'Fetching data with headers {headers} from {request_url}')

                try:
                    resp = requests.get(request_url, headers=headers)
                    resp.raise_for_status()
                except requests.exceptions.RequestException as e:
                    logging.error(f"Error fetching data: {e}")
                    raise OpentableDownloaderError("Failed to fetch data from OpenTable.")

                data = resp.json()
                for rest in data.get('items', []):
                    f.write(json.dumps(rest) + '\n')

                total_items = int(data.get('total_items', 0))
                items_count = len(data.get('items', []))

                if total_items <= offset + items_count:
                    break

                offset += items_count

    def _get_token(self):
        url = 'https://oauth.opentable.com/api/v2/oauth/token'
        headers = self._add_auth_header({})
        data = {'grant_type': 'client_credentials'}

        try:
            resp = requests.post(url, headers=headers, data=data)
            resp.raise_for_status()
        except requests.exceptions.RequestException as e:
            logging.error(f"Error fetching token: {e}")
            raise OpentableDownloaderError("Can't get token from OpenTable.")

        self.token = resp.json()
        logging.debug(f'Token is {self.token}')

    def _add_auth_header(self, headers):
        if self.token is None:
            credentials = base64.b64encode(f'{self.login}:{self.password}'.encode('utf-8')).decode('utf-8')
            headers['Authorization'] = f'Basic {credentials}'
        else:
            headers['Authorization'] = f"{self.token['token_type']} {self.token['access_token']}"
        return headers


def make_tsv(data_file, output_file):
    for line in data_file:
        rest = json.loads(line)
        try:
            address = ' '.join([rest.get('address', ''), rest.get('city', ''), rest.get('country', '')])
            # Clean up any extra whitespace characters
            address = re.sub(r'\s+', ' ', address)
        except TypeError:
            address = ''

        row = '\t'.join([
            str(rest.get('rid', '')),
            str(rest.get('latitude', '')),
            str(rest.get('longitude', '')),
            rest.get('name', ''),
            address,
            rest.get('reservation_url', ''),
            rest.get('phone_number', '')
        ])

        output_file.write(row + '\n')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description='Downloads OpenTable data.')
    parser.add_argument('-d', '--download', action='store_true', help='Download data from OpenTable')
    parser.add_argument('--tsv', type=str, nargs='?', const='',
                        help='File to save TSV data (stdout if not provided)')
    parser.add_argument('--opentable_data', type=str, help='Path to the OpenTable data file')

    # Opentable client credentials
    parser.add_argument('--client', required=True, help='OpenTable client ID')
    parser.add_argument('--secret', required=True, help='OpenTable client secret')

    args = parser.parse_args()

    if args.download:
        logging.info('Downloading OpenTable data...')
        loader = OpentableDownloader(args.client, args.secret, args.opentable_data)
        loader.download()

    if args.tsv is not None:
        with open(args.opentable_data, 'r', encoding='utf-8') as data_file:
            output = open(args.tsv, 'w', encoding='utf-8') if args.tsv else sys.stdout
            make_tsv(data_file, output)
            if args.tsv:
                output.close()
