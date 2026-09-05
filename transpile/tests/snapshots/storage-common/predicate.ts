// MIRRORS: ankurah/storage/common/src/predicate.rs
import { Struct } from '@ankurah/base';
import { Predicate } from '@ankurah/ankql';
import { Comparison } from '@ankurah/core';

export class ConjunctFinder extends Struct {

  static find(predicate: Predicate): Predicate[] {
    let conjuncts = [];
    ConjunctFinder.Self.extractConjuncts(predicate, conjuncts);
    return conjuncts;
  }

  static extractConjuncts(predicate: Predicate, conjuncts: Predicate[]): void {
    return predicate.match({
      And: (v) => {
        const left = v._0;
        const right = v._1;
        ConjunctFinder.Self.extractConjuncts(left, conjuncts);
        ConjunctFinder.Self.extractConjuncts(right, conjuncts);
      },
      Or: (v) => {
        conjuncts.push(predicate.clone());
      },
      Comparison: () => {
        conjuncts.push(predicate.clone());
      },
      IsNull: () => {
        conjuncts.push(predicate.clone());
      },
      Not: () => {
        conjuncts.push(predicate.clone());
      },
      True: () => {
        conjuncts.push(predicate.clone());
      },
      False: () => {
        conjuncts.push(predicate.clone());
      },
      Placeholder: () => {
        conjuncts.push(predicate.clone());
      },
    });
  }
}

