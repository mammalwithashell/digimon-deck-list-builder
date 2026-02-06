using System;
using System.Collections.Generic;

namespace Digimon.Core.Loggers
{
    public interface IGameLogger
    {
        void Log(string message);
        void LogVerbose(string message);
        List<string> GetLogs();
        void Clear();
    }

    public class SilentLogger : IGameLogger
    {
        // No-op implementation for maximum performance
        public void Log(string message) { }
        public void LogVerbose(string message) { }
        public List<string> GetLogs() => new List<string>();
        public void Clear() { }
    }

    public class VerboseLogger : IGameLogger
    {
        private readonly List<string> _logs = new List<string>();
        
        public void Log(string message)
        {
            _logs.Add(message);
            // Optionally still write to console for immediate debugging if needed
            // Console.WriteLine(message); 
        }

        public void LogVerbose(string message)
        {
            _logs.Add($"[VERBOSE] {message}");
        }

        public List<string> GetLogs()
        {
            return new List<string>(_logs);
        }

        public void Clear()
        {
            _logs.Clear();
        }
    }
}
