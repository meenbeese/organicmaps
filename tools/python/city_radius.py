import math
import sys
from optparse import OptionParser

import matplotlib.pyplot as plt

cities = []

def strip(s):
    return s.strip('\t\n ')

def load_data(path):
    global cities
    with open(path, 'r') as f:
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
            'square': float(data[5]),
        }
        cities.append(item)

    # build plot
    print(f"Cities count: {len(cities)}")

def formula(popul, base=32, mult=0.5):
    return math.pow(popul, 1 / base) * mult

def avgDistance(approx, data):
    dist = 0
    for x in range(len(data)):
        dist += abs(approx[x] - data[x])
    return dist / len(data)

def findBest(popul, data, minBase=5, maxBase=100, stepBase=0.1, minMult=0.01, maxMult=1, stepMult=0.01):
    base = minBase
    minDist = -1
    bestMult = minMult
    bestBase = base

    while base <= maxBase:
        print(f"{100 * (base - minBase) / (maxBase - minBase):.02f}% best mult: {bestMult}, best base: {bestBase}, best dist: {minDist}")
        mult = minMult

        while mult <= maxMult:
            approx = [formula(p, base, mult) for p in popul]
            dist = avgDistance(approx, data)

            if minDist < 0 or minDist > dist:
                minDist = dist
                bestBase = base
                bestMult = mult

            mult += stepMult

        base += stepBase

    return bestBase, bestMult

def process_data(steps_count, base, mult, bestFind=False, dataFlag=0):
    avgData, maxData, sqrData, population = [], [], [], []
    maxPopulation, minPopulation = 0, -1

    for city in cities:
        p, w, h, s = city['population'], city['width'], city['height'], city['square']
        population.append(p)
        maxPopulation = max(maxPopulation, p)
        minPopulation = p if minPopulation < 0 else min(minPopulation, p)
        maxData.append(max([w, h]))
        avgData.append((w + h) * 0.5)
        sqrData.append(math.sqrt(s))

    bestBase, bestMult = base, mult
    if bestFind:
        d = maxData if dataFlag == 0 else avgData if dataFlag == 1 else sqrData
        bestBase, bestMult = findBest(population, d)

    print(f"Finished\n\nBest mult: {bestMult}, Best base: {bestBase}")

    approx, population2 = [], []
    v = minPopulation
    step = (maxPopulation - minPopulation) / steps_count
    for _ in range(steps_count):
        approx.append(formula(v, bestBase, bestMult))
        population2.append(v)
        v += step

    plt.plot(population, avgData, 'bo', label='Avg Data')
    plt.plot(population, maxData, 'ro', label='Max Data')
    plt.plot(population, sqrData, 'go', label='Sqr Data')
    plt.plot(population2, approx, 'y-', label='Approximation')
    plt.axis([minPopulation, maxPopulation, 0, 100])
    plt.xscale('log')
    plt.grid(True)
    plt.legend()
    plt.show()

if __name__ == "__main__":
    parser = OptionParser()
    parser.add_option("-f", "--file", dest="filename", default="city_popul_sqr.data",
                      help="source data file", metavar="path")
    parser.add_option("-s", "--scan", dest="best", action="store_true", default=False,
                      help="scan best values of mult and base")
    parser.add_option('-m', "--mult", dest='mult', type="float", default=1,
                      help='multiplier value')
    parser.add_option('-b', '--base', dest='base', type="float", default=3.6,
                      help="base value")
    parser.add_option('-d', '--data', dest='data', type="int", default=0,
                      help="Dataset to use on best values scan: 0 - max, 1 - avg, 2 - sqr")

    (options, args) = parser.parse_args()

    if not options.filename:
        print('Usage: city_radius.py -f <data_file> -s')
        sys.exit(1)

    load_data(options.filename)
    process_data(1000, options.base, options.mult, options.best, options.data)
