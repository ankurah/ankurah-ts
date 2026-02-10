// MIRRORS: ankurah/proto/src/human_id.rs
//
// Human-readable identifier generation from byte arrays.
// Uses a fixed 256-word dictionary (matching Rust WORDLIST).

const WORDLIST: readonly string[] = [
  'ack', 'alabama', 'alanine', 'alaska', 'alpha', 'angel', 'apart', 'april',
  'arizona', 'arkansas', 'artist', 'asparagus', 'aspen', 'august', 'autumn',
  'avocado', 'bacon', 'bakerloo', 'batman', 'beer', 'berlin', 'beryllium',
  'black', 'blossom', 'blue', 'bluebird', 'bravo', 'bulldog', 'burger',
  'butter', 'california', 'carbon', 'cardinal', 'carolina', 'carpet', 'cat',
  'ceiling', 'charlie', 'chicken', 'coffee', 'cola', 'cold', 'colorado',
  'comet', 'connecticut', 'crazy', 'cup', 'dakota', 'december', 'delaware',
  'delta', 'diet', 'don', 'double', 'early', 'earth', 'east', 'echo',
  'edward', 'eight', 'eighteen', 'eleven', 'emma', 'enemy', 'equal',
  'failed', 'fanta', 'fifteen', 'fillet', 'finch', 'fish', 'five', 'fix',
  'floor', 'florida', 'football', 'four', 'fourteen', 'foxtrot', 'freddie',
  'friend', 'fruit', 'gee', 'georgia', 'glucose', 'golf', 'green', 'grey',
  'hamper', 'happy', 'harry', 'hawaii', 'helium', 'high', 'hot', 'hotel',
  'hydrogen', 'idaho', 'illinois', 'india', 'indigo', 'ink', 'iowa',
  'island', 'item', 'jersey', 'jig', 'johnny', 'juliet', 'july', 'jupiter',
  'kansas', 'kentucky', 'kilo', 'king', 'kitten', 'lactose', 'lake', 'lamp',
  'lemon', 'leopard', 'lima', 'lion', 'lithium', 'london', 'louisiana',
  'low', 'magazine', 'magnesium', 'maine', 'mango', 'march', 'mars',
  'maryland', 'massachusetts', 'may', 'mexico', 'michigan', 'mike',
  'minnesota', 'mirror', 'mississippi', 'missouri', 'mobile', 'mockingbird',
  'monkey', 'montana', 'moon', 'mountain', 'muppet', 'music', 'nebraska',
  'neptune', 'network', 'nevada', 'nine', 'nineteen', 'nitrogen', 'north',
  'november', 'nuts', 'october', 'ohio', 'oklahoma', 'one', 'orange',
  'oranges', 'oregon', 'oscar', 'oven', 'oxygen', 'papa', 'paris', 'pasta',
  'pennsylvania', 'pip', 'pizza', 'pluto', 'potato', 'princess', 'purple',
  'quebec', 'queen', 'quiet', 'red', 'river', 'robert', 'robin', 'romeo',
  'rugby', 'sad', 'salami', 'saturn', 'september', 'seven', 'seventeen',
  'shade', 'sierra', 'single', 'sink', 'six', 'sixteen', 'skylark', 'snake',
  'social', 'sodium', 'solar', 'south', 'spaghetti', 'speaker', 'spring',
  'stairway', 'steak', 'stream', 'summer', 'sweet', 'table', 'tango', 'ten',
  'tennessee', 'tennis', 'texas', 'thirteen', 'three', 'timing', 'triple',
  'twelve', 'twenty', 'two', 'uncle', 'undress', 'uniform', 'uranus', 'utah',
  'vegan', 'venus', 'vermont', 'victor', 'video', 'violet', 'virginia',
  'washington', 'west', 'whiskey', 'white', 'william', 'winner', 'winter',
  'wisconsin', 'wolfram', 'wyoming', 'xray', 'yankee', 'yellow', 'zebra',
  'zulu',
] as const;

/**
 * Compress bytes into a shorter array by XOR-folding segments.
 */
function compress(bytes: Uint8Array | readonly number[], target: number): number[] {
  const segSize = Math.floor(bytes.length / target);
  const result: number[] = [];
  for (let i = 0; i < target; i++) {
    const start = i * segSize;
    const end = Math.min(start + segSize, bytes.length);
    let xor = 0;
    for (let j = start; j < end; j++) {
      xor ^= bytes[j];
    }
    result.push(xor);
  }
  return result;
}

/**
 * Generate a human-readable identifier from bytes.
 * Matches Rust `humanize()` function.
 */
export function humanize(bytes: Uint8Array | readonly number[], wordsOut: number): string {
  const compressed = compress(bytes, wordsOut);
  return compressed.map(x => WORDLIST[x]).join('-');
}

/**
 * Convert bytes to hex string.
 * Matches Rust `hex()` function.
 */
export function hex(bytes: Uint8Array | readonly number[]): string {
  return Array.from(bytes).map(x => x.toString(16)).join('');
}
