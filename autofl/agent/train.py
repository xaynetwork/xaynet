from collections import deque
from typing import Deque, List

import gym
import numpy as np
from absl import logging

from .agent import Agent


# pylint: disable-msg=too-many-arguments,too-many-locals
def train(
    agent: Agent,
    env: gym.Env,
    max_e: int = 2000,
    max_t: int = 1000,
    epsilon_initial=1.0,
    epsilon_final=0.1,
    discrete=True,
):
    logging.info("env.action_space: {}".format(env.action_space))
    logging.info("env.observation_space: {}".format(env.observation_space))

    # Initialize metrics
    episode_rewards: List[float] = []
    episode_rewards_window: Deque = deque(maxlen=100)
    reward_min = 0
    reward_max = 0
    epsilon = epsilon_initial

    # Train for max_e episodes
    for e in range(1, max_e + 1):
        observation = env.reset()
        episode_reward = 0
        # Start episode
        for _ in range(max_t):
            # Select action
            if discrete:
                action = agent.action_discrete(observation, epsilon)
            else:
                action = agent.action_multi_discrete(observation, epsilon)
            # Take action
            next_observation, reward, done, _ = env.step(action)
            # Update agent with experience
            transition = (observation, action, reward, next_observation, done)
            agent.update(transition)
            # Update observation
            observation = next_observation
            # Record reward
            episode_reward += reward
            reward_min = min(reward_min, reward)
            reward_max = max(reward_max, reward)
            # Stop if state is terminal
            if done:
                break

        # Record reward(s)
        episode_rewards.append(episode_reward)
        episode_rewards_window.append(episode_reward)
        mean_episode_reward = np.mean(episode_rewards_window)
        solved = mean_episode_reward >= 200.0
        # Log progress
        log_progress(e, mean_episode_reward, reward_min, reward_max, epsilon, solved)
        # Decrease epsilon
        epsilon = max(epsilon_final, epsilon * 0.995)
        # Stop training and save model
        if solved:
            agent.save_policy()
            break


# pylint: disable-msg=too-many-arguments
def log_progress(
    episode: int,
    mean_episode_reward: float,
    reward_min: float,
    reward_max: float,
    epsilon: float,
    solved=False,
):
    # TODO migrate to absl
    line_terminator = "\n" if episode % 100 == 0 else ""
    print(
        "\rEpisode {}\tAvg Score: {:.2f}\te: {:.3f}\tr_max: {:.1f}\tr_min: {:.1f}".format(
            episode, mean_episode_reward, epsilon, reward_max, reward_min
        ),
        end=line_terminator,
    )
    if solved:
        print(
            "\nEnvironment solved in {:d} episodes!\tAverage Score: {:.2f}".format(
                episode - 100, mean_episode_reward
            )
        )


def watch(agent, env, epsilon=0.0, episodes=1, max_t=200):
    for _ in range(episodes):
        observation = env.reset()
        for _ in range(max_t):
            env.render()
            # Select action
            if isinstance(env.action_space, gym.spaces.discrete.Discrete):
                action = agent.action_discrete(observation, epsilon)
            elif isinstance(env.action_space, gym.spaces.multi_discrete.MultiDiscrete):
                action = agent.action_multi_discrete(observation, epsilon)
            observation, _, done, _ = env.step(action)
            if done:
                break
    env.close()
