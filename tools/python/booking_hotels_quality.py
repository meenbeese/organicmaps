#!/usr/bin/env python3
# coding: utf-8

from sklearn import metrics
import argparse
import logging
import matplotlib.pyplot as plt
import re

# Initialize logging
logging.basicConfig(level=logging.DEBUG, format='[%(asctime)s] %(levelname)s: %(message)s')


def load_binary_list(path):
    """Loads reference binary classifier output."""
    bits = []
    with open(path, 'r', encoding='utf-8') as fd:
        for line in fd:
            if (not line.strip()) or line.startswith('#'):
                continue
            bits.append(1 if line.startswith('y') else 0)
    return bits


def load_score_list(path):
    """Loads list of matching scores."""
    scores = []
    with open(path, 'r', encoding='utf-8') as fd:
        for line in fd:
            if (not line.strip()) or line.startswith('#'):
                continue
            match = re.search(r'result score: (\d*\.\d+)', line)
            if match:
                scores.append(float(match.group(1)))
    return scores


def process_options():
    parser = argparse.ArgumentParser(description="Download and process booking hotels.")
    parser.add_argument("-v", "--verbose", action="store_true", dest="verbose")
    parser.add_argument("-q", "--quiet", action="store_false", dest="verbose")

    parser.add_argument("--reference_list", dest="reference_list", help="Path to reference data file", required=True)
    parser.add_argument("--sample_list", dest="sample_list", help="Path to sample data file", required=True)

    parser.add_argument("--show", dest="show", default=False, action="store_true",
                        help="Show graph for precision and recall")

    return parser.parse_args()


def main():
    options = process_options()
    reference = load_binary_list(options.reference_list)
    sample = load_score_list(options.sample_list)

    precision, recall, thresholds = metrics.precision_recall_curve(reference, sample)
    pr_thresholds = zip(precision[:-1], recall[:-1], thresholds)  # Last threshold isn't used

    # Find optimal threshold using harmonic mean (F1-like metric)
    max_by_hmean = max(pr_thresholds, key=lambda prt: prt[0] * prt[1] / (prt[0] + prt[1]))
    print(f"Optimal threshold: {max_by_hmean[2]} for precision: {max_by_hmean[0]} and recall: {max_by_hmean[1]}")
    print(f"AUC: {metrics.roc_auc_score(reference, sample)}")

    if options.show:
        plt.plot(recall, precision)
        plt.title("Precision/Recall")
        plt.ylabel("Precision")
        plt.xlabel("Recall")
        plt.show()


if __name__ == "__main__":
    main()
