using System;
using System.Collections.Generic;

namespace Digimon.Core
{
    public static class CardRegistry
    {
        private static Dictionary<string, int> _idToInt = new Dictionary<string, int>();
        private static Dictionary<int, string> _intToId = new Dictionary<int, string>();
        private static int _nextId = 1;

        public static void Register(string cardId)
        {
            if (!_idToInt.ContainsKey(cardId))
            {
                _idToInt[cardId] = _nextId;
                _intToId[_nextId] = cardId;
                _nextId++;
            }
        }

        public static int GetId(string cardId)
        {
            if (string.IsNullOrEmpty(cardId)) return 0;
            if (_idToInt.TryGetValue(cardId, out int id)) return id;
            return 0; // Unknown or Empty
        }

        public static string GetStringId(int internalId)
        {
            if (_intToId.TryGetValue(internalId, out string id)) return id;
            return null;
        }

        public static void Reset()
        {
            _idToInt.Clear();
            _intToId.Clear();
            _nextId = 1;
            // Pre-register dummy cards for testing
            Register("DebugCard_001");
            Register("Card1");
        }

        static CardRegistry()
        {
            Reset();
        }
    }
}
