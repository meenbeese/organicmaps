#!/usr/bin/env python3
import sys
import math
import matplotlib.pyplot as plt
from optparse import OptionParser

cities = []


def strip(s):
    return s.strip('\t\n ')


def load_data(path):
    global cities

    with open(path, 'r', encoding='utf-8') as f:
        lines = f.readlines()

    for l in lines:
        if l.startswith('#'):
            continue

        data = l.split('|')

        if len(data) < 6:
            continue

        item = {
            'name': strip(data[0]),
            'population': int(strip(data[1])),
            'region': strip(data[2]),
            'width': float(strip(data[3])),
            'height': float(strip(data[4])),
            'square': float(data[5])
        }

        cities.append(item)

    # build plot
    print("Cities count: {}".format(len(cities)))


def formula(popul, base=32, mult=0.5):
    return math.pow(popul, 1 / base) * mult


def avgDistance(approx, data):
    dist = sum(abs(approx[i] - data[i]) for i in range(len(data)))
    return dist / float(len(data))


def findBest(popul, data, minBase=5, maxBase=100, stepBase=0.1, minMult=0.01, maxMult=1, stepMult=0.01):
    base = minBase
    minDist = float('inf')
    bestMult = minMult
    bestBase = base

    while base <= maxBase:
        print("{:.02f}% best mult: {:.6f}, best base: {:.6f}, best dist: {:.6f}".format(
            100 * (base - minBase) / (maxBase - minBase), bestMult, bestBase, minDist))

        mult = minMult

        while mult <= maxMult:
            approx = [formula(p, base, mult) for p in popul]
            dist = avgDistance(approx, data)

            if dist < minDist:
                minDist = dist
                bestBase = base
                bestMult = mult

            mult += stepMult

        base += stepBase

    return bestBase, bestMult


def process_data(steps_count, base, mult, bestFind=False, dataFlag=0):
    avgData = []
    maxData = []
    sqrData = []
    population = []
    maxPopulation = 0
    minPopulation = float('inf')

    for city in cities:
        p = city['population']
        w = city['width']
        h = city['height']
        s = city['square']
        population.append(p)

        maxPopulation = max(maxPopulation, p)
        minPopulation = min(minPopulation, p)

        maxData.append(max([w, h]))
        avgData.append((w + h) * 0.5)
        sqrData.append(math.sqrt(s))

    bestBase, bestMult = base, mult

    if bestFind:
        d = maxData if dataFlag == 0 else avgData if dataFlag == 1 else sqrData
        bestBase, bestMult = findBest(population, d)

    print("\nFinished\nBest mult: {:.6f}, Best base: {:.6f}".format(bestMult, bestBase))

    approx = []
    population2 = []
    step = (maxPopulation - minPopulation) / float(steps_count)
    v = minPopulation

    for _ in range(steps_count):
        approx.append(formula(v, bestBase, bestMult))
        population2.append(v)
        v += step

    plt.plot(population, avgData, 'bo', population, maxData, 'ro', population, sqrData, 'go', population2, approx, 'y')
    plt.axis([minPopulation, maxPopulation, 0, 100])
    plt.xscale('log')
    plt.show()


if __name__ == "__main__":
    if len(sys.argv) < 3:
        print('Usage: city_radius.py <data_file> <steps>')
        sys.exit(1)

    parser = OptionParser()
    parser.add_option("-f", "--file", dest="filename", default="city_popul_sqr.data", help="source data file")
    parser.add_option("-s", "--scan", dest="best", default=False, action="store_true", help="scan best values of mult and base")
    parser.add_option('-m', "--mult", dest='mult', default=1, help='multiplier value', type='float')
    parser.add_option('-b', '--base', dest='base', default=3.6, help="base value", type='float')
    parser.add_option('-d', '--data', default=0, dest='data', help="Dataset to use on best values scan: 0 - max, 1 - avg, 2 - sqr", type='int')

    (options, args) = parser.parse_args()
    load_data(options.filename)
    process_data(1000, float(options.base), float(options.mult), options.best, int(options.data))
