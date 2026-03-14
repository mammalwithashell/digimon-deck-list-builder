"""ArchitectDQN — DQN agent with Prioritized Experience Replay for deck optimization.

Pure PyTorch implementation with no game engine dependencies. Provides:
- SumTree for O(log n) prioritized sampling
- PrioritizedReplayBuffer with importance-sampling correction
- QNetwork (2-hidden-layer MLP)
- ArchitectDQN with Double DQN updates, epsilon-greedy action masking, and save/load
"""

from __future__ import annotations

from typing import Any, Dict, List, Optional, Tuple

import numpy as np
import torch
import torch.nn as nn
import torch.optim as optim


class SumTree:
    """Binary tree for O(log n) prioritized sampling.

    Leaf nodes store priorities. Internal nodes store the sum of their children.
    Total capacity is fixed at construction time; writes wrap around.
    """

    def __init__(self, capacity: int):
        self.capacity = capacity
        self._tree = np.zeros(2 * capacity - 1, dtype=np.float64)
        self._data: List[Any] = [None] * capacity
        self._write_idx = 0
        self._size = 0

    # -- public interface -----------------------------------------------------

    def add(self, priority: float, data: Any) -> None:
        tree_idx = self._write_idx + self.capacity - 1
        self._data[self._write_idx] = data
        self._update(tree_idx, priority)
        self._write_idx = (self._write_idx + 1) % self.capacity
        self._size = min(self._size + 1, self.capacity)

    def update(self, tree_idx: int, priority: float) -> None:
        self._update(tree_idx, priority)

    def sample(self, batch_size: int) -> Tuple[List[int], List[Any], List[float]]:
        """Segment-based proportional sampling.

        Returns ``(tree_indices, data_list, priorities)``.
        """
        indices: List[int] = []
        data_list: List[Any] = []
        priorities: List[float] = []

        segment = self.total / batch_size
        for i in range(batch_size):
            low = segment * i
            high = segment * (i + 1)
            value = np.random.uniform(low, high)
            idx, pri, data = self._retrieve(value)
            indices.append(idx)
            data_list.append(data)
            priorities.append(pri)

        return indices, data_list, priorities

    @property
    def total(self) -> float:
        return float(self._tree[0])

    @property
    def min_priority(self) -> float:
        leaf_start = self.capacity - 1
        active = self._tree[leaf_start : leaf_start + self._size]
        if len(active) == 0:
            return 0.0
        return float(active.min())

    @property
    def size(self) -> int:
        return self._size

    # -- internals ------------------------------------------------------------

    def _update(self, tree_idx: int, priority: float) -> None:
        delta = priority - self._tree[tree_idx]
        self._tree[tree_idx] = priority
        while tree_idx > 0:
            tree_idx = (tree_idx - 1) // 2
            self._tree[tree_idx] += delta

    def _retrieve(self, value: float) -> Tuple[int, float, Any]:
        idx = 0
        while True:
            left = 2 * idx + 1
            right = left + 1
            if left >= len(self._tree):
                break
            if value <= self._tree[left]:
                idx = left
            else:
                value -= self._tree[left]
                idx = right
        data_idx = idx - (self.capacity - 1)
        return idx, float(self._tree[idx]), self._data[data_idx]


class PrioritizedReplayBuffer:
    """Prioritized Experience Replay buffer backed by a :class:`SumTree`."""

    def __init__(
        self,
        capacity: int = 100_000,
        alpha: float = 0.6,
        beta: float = 0.4,
        beta_increment: float = 0.001,
        epsilon: float = 1e-6,
    ):
        self.capacity = capacity
        self.alpha = alpha
        self.beta = beta
        self.beta_increment = beta_increment
        self.epsilon = epsilon

        self._tree = SumTree(capacity)
        self._max_priority = 1.0

    # -- public interface -----------------------------------------------------

    def add(
        self,
        state: np.ndarray,
        action: int,
        reward: float,
        next_state: np.ndarray,
        done: bool,
        action_mask: np.ndarray,
        next_action_mask: np.ndarray,
    ) -> None:
        transition = (
            state,
            action,
            reward,
            next_state,
            done,
            action_mask,
            next_action_mask,
        )
        priority = self._max_priority ** self.alpha
        self._tree.add(priority, transition)

    def sample(
        self, batch_size: int
    ) -> Tuple[Dict[str, np.ndarray], np.ndarray, np.ndarray]:
        """Sample a batch with importance-sampling weights.

        Returns ``(batch_dict, tree_indices, importance_weights)``.
        """
        indices, data_list, priorities = self._tree.sample(batch_size)

        states, actions, rewards, next_states, dones, masks, next_masks = zip(
            *data_list
        )

        batch = {
            "states": np.array(states, dtype=np.float32),
            "actions": np.array(actions, dtype=np.int64),
            "rewards": np.array(rewards, dtype=np.float32),
            "next_states": np.array(next_states, dtype=np.float32),
            "dones": np.array(dones, dtype=np.float32),
            "action_masks": np.array(masks, dtype=np.float32),
            "next_action_masks": np.array(next_masks, dtype=np.float32),
        }

        tree_indices = np.array(indices, dtype=np.int64)
        pri_arr = np.array(priorities, dtype=np.float64)

        # Importance-sampling weights
        n = self._tree.size
        min_prob = (self._tree.min_priority + 1e-12) / (self._tree.total + 1e-12)
        probs = pri_arr / (self._tree.total + 1e-12)
        weights = (n * probs) ** (-self.beta)
        max_weight = (n * min_prob) ** (-self.beta)
        weights /= max_weight + 1e-12

        # Anneal beta toward 1.0
        self.beta = min(1.0, self.beta + self.beta_increment)

        return batch, tree_indices, weights.astype(np.float32)

    def update_priorities(self, indices: np.ndarray, td_errors: np.ndarray) -> None:
        priorities = (np.abs(td_errors) + self.epsilon) ** self.alpha
        for idx, pri in zip(indices, priorities):
            self._tree.update(int(idx), float(pri))
            self._max_priority = max(
                self._max_priority, float(pri) ** (1.0 / self.alpha)
            )

    @property
    def size(self) -> int:
        return self._tree.size


class QNetwork(nn.Module):
    """Two-hidden-layer MLP Q-network."""

    def __init__(self, state_dim: int, action_dim: int, hidden_size: int = 1024):
        super().__init__()
        self.net = nn.Sequential(
            nn.Linear(state_dim, hidden_size),
            nn.ReLU(),
            nn.Linear(hidden_size, hidden_size),
            nn.ReLU(),
            nn.Linear(hidden_size, action_dim),
        )

    def forward(self, state: torch.Tensor) -> torch.Tensor:
        """Return Q-values for all actions.  Shape: ``(batch, action_dim)``."""
        return self.net(state)


class ArchitectDQN:
    """DQN agent for deck optimization with prioritized experience replay.

    Uses Double DQN for target computation and epsilon-greedy exploration
    with action masking.
    """

    def __init__(
        self,
        state_dim: int,
        action_dim: int,
        hidden_size: int = 1024,
        lr: float = 1e-4,
        gamma: float = 0.99,
        epsilon_start: float = 1.0,
        epsilon_end: float = 0.2,
        epsilon_decay: float = 0.0005,
        replay_capacity: int = 100_000,
        batch_size: int = 64,
        target_update_freq: int = 100,
        device: str = "auto",
    ):
        # Hyperparameters
        self._state_dim = state_dim
        self._action_dim = action_dim
        self._hidden_size = hidden_size
        self._lr = lr
        self._gamma = gamma
        self._epsilon_start = epsilon_start
        self._epsilon_end = epsilon_end
        self._epsilon_decay = epsilon_decay
        self._replay_capacity = replay_capacity
        self._batch_size = batch_size
        self._target_update_freq = target_update_freq

        # Device
        if device == "auto":
            self._device = torch.device(
                "cuda" if torch.cuda.is_available() else "cpu"
            )
        else:
            self._device = torch.device(device)

        # Networks
        self._online_net = QNetwork(state_dim, action_dim, hidden_size).to(
            self._device
        )
        self._target_net = QNetwork(state_dim, action_dim, hidden_size).to(
            self._device
        )
        self.update_target_network()
        self._target_net.eval()

        # Optimizer
        self._optimizer = optim.Adam(self._online_net.parameters(), lr=lr)

        # Replay buffer
        self._buffer = PrioritizedReplayBuffer(capacity=replay_capacity)

        # Exploration state
        self._epsilon = epsilon_start
        self._episodes_trained = 0
        self._update_steps = 0

    # -- action selection -----------------------------------------------------

    def select_action(self, state: np.ndarray, action_mask: np.ndarray) -> int:
        """Epsilon-greedy action selection with masking.

        During exploration (random): sample uniformly from valid actions.
        During exploitation (greedy): pick the valid action with highest Q-value.
        """
        if np.random.random() < self._epsilon:
            valid = np.where(action_mask > 0)[0]
            return int(np.random.choice(valid))
        return self.greedy_action(state, action_mask)

    def greedy_action(self, state: np.ndarray, action_mask: np.ndarray) -> int:
        """Pure greedy action selection (inference mode). No exploration."""
        with torch.no_grad():
            state_t = torch.as_tensor(
                state, dtype=torch.float32, device=self._device
            ).unsqueeze(0)
            q_values = self._online_net(state_t).squeeze(0)
            mask_t = torch.as_tensor(
                action_mask, dtype=torch.bool, device=self._device
            )
            q_values[~mask_t] = -float("inf")
            return int(q_values.argmax().item())

    # -- replay buffer --------------------------------------------------------

    def store_transition(
        self,
        state: np.ndarray,
        action: int,
        reward: float,
        next_state: np.ndarray,
        done: bool,
        action_mask: np.ndarray,
        next_action_mask: np.ndarray,
    ) -> None:
        """Store a transition in the replay buffer."""
        self._buffer.add(
            state, action, reward, next_state, done, action_mask, next_action_mask
        )

    # -- learning -------------------------------------------------------------

    def update(self) -> Optional[float]:
        """Sample a batch from PER and perform one Double DQN gradient step.

        Returns the mean loss, or ``None`` if the buffer is too small.
        """
        if self._buffer.size < self._batch_size:
            return None

        batch, tree_indices, weights = self._buffer.sample(self._batch_size)

        # Move to device
        states = torch.as_tensor(batch["states"], device=self._device)
        actions = torch.as_tensor(batch["actions"], device=self._device).unsqueeze(1)
        rewards = torch.as_tensor(batch["rewards"], device=self._device).unsqueeze(1)
        next_states = torch.as_tensor(batch["next_states"], device=self._device)
        dones = torch.as_tensor(batch["dones"], device=self._device).unsqueeze(1)
        next_masks = torch.as_tensor(
            batch["next_action_masks"], dtype=torch.bool, device=self._device
        )
        is_weights = torch.as_tensor(weights, device=self._device).unsqueeze(1)

        # Current Q-values
        q_current = self._online_net(states).gather(1, actions)

        # Double DQN target
        with torch.no_grad():
            # Action selection from online network
            online_next_q = self._online_net(next_states)
            online_next_q[~next_masks] = -1e9
            next_actions = online_next_q.argmax(dim=1, keepdim=True)

            # Value estimation from target network
            q_next = self._target_net(next_states).gather(1, next_actions)
            q_target = rewards + self._gamma * q_next * (1.0 - dones)

        # TD error
        td_error = q_current - q_target

        # Weighted MSE loss
        loss = (is_weights * td_error.pow(2)).mean()

        # Gradient step
        self._optimizer.zero_grad()
        loss.backward()
        self._optimizer.step()

        # Update priorities
        self._buffer.update_priorities(
            tree_indices, td_error.detach().cpu().squeeze(1).numpy()
        )

        # Periodic target network update
        self._update_steps += 1
        if self._update_steps % self._target_update_freq == 0:
            self.update_target_network()

        return float(loss.item())

    def decay_epsilon(self) -> None:
        """Decay epsilon by ``epsilon_decay``. Call once per episode."""
        self._epsilon = max(
            self._epsilon_end, self._epsilon - self._epsilon_decay
        )
        self._episodes_trained += 1

    def update_target_network(self) -> None:
        """Hard copy online network weights to target network."""
        self._target_net.load_state_dict(self._online_net.state_dict())

    # -- properties -----------------------------------------------------------

    @property
    def epsilon(self) -> float:
        return self._epsilon

    @property
    def episodes_trained(self) -> int:
        return self._episodes_trained

    # -- persistence ----------------------------------------------------------

    def save(self, path: str) -> None:
        """Save model state and hyperparameters to a ``.pt`` file."""
        torch.save(
            {
                "online_net": self._online_net.state_dict(),
                "target_net": self._target_net.state_dict(),
                "optimizer": self._optimizer.state_dict(),
                "epsilon": self._epsilon,
                "episodes_trained": self._episodes_trained,
                "update_steps": self._update_steps,
                "hyperparameters": self.get_config(),
            },
            path,
        )

    @classmethod
    def load(cls, path: str, device: str = "auto") -> "ArchitectDQN":
        """Load a saved model, reconstructing the agent from saved hyperparameters."""
        checkpoint = torch.load(path, map_location="cpu", weights_only=False)
        hp = checkpoint["hyperparameters"]

        agent = cls(
            state_dim=hp["state_dim"],
            action_dim=hp["action_dim"],
            hidden_size=hp["hidden_size"],
            lr=hp["lr"],
            gamma=hp["gamma"],
            epsilon_start=hp["epsilon_start"],
            epsilon_end=hp["epsilon_end"],
            epsilon_decay=hp["epsilon_decay"],
            replay_capacity=hp["replay_capacity"],
            batch_size=hp["batch_size"],
            target_update_freq=hp["target_update_freq"],
            device=device,
        )

        agent._online_net.load_state_dict(checkpoint["online_net"])
        agent._target_net.load_state_dict(checkpoint["target_net"])
        agent._optimizer.load_state_dict(checkpoint["optimizer"])
        agent._epsilon = checkpoint["epsilon"]
        agent._episodes_trained = checkpoint["episodes_trained"]
        agent._update_steps = checkpoint.get("update_steps", 0)

        return agent

    def get_config(self) -> Dict[str, Any]:
        """Return hyperparameters as a serializable dict."""
        return {
            "state_dim": self._state_dim,
            "action_dim": self._action_dim,
            "hidden_size": self._hidden_size,
            "lr": self._lr,
            "gamma": self._gamma,
            "epsilon_start": self._epsilon_start,
            "epsilon_end": self._epsilon_end,
            "epsilon_decay": self._epsilon_decay,
            "replay_capacity": self._replay_capacity,
            "batch_size": self._batch_size,
            "target_update_freq": self._target_update_freq,
        }
