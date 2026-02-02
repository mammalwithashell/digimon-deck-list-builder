from python_impl.digimon_gym import GameState

def verify():
    print("Initializing Gym Wrapper...")
    env = GameState()
    
    print("Resetting Environment...")
    obs = env.reset()
    
    tensor = obs.get("tensor")
    if tensor is None:
        raise ValueError("Observation tensor missing!")
    
    print(f"Observation Tensor Shape: {tensor.shape}")
    
    if len(tensor) < 500:
       print(f"Warning: Tensor size seems small: {len(tensor)}. Expected ~680+")
    else:
       print(f"Tensor size check passed: {len(tensor)}")
       
    print("Running 10 Random Steps...")
    for i in range(10):
        # Action 62 is Pass Turn in new decoder
        action_id = 62 
        # Try Play Card 0 on some turns
        if i % 3 == 0: action_id = 0
            
        print(f"Step {i+1}: Action {action_id}")
        obs, reward, done, info = env.step(action_id)
        
        if done:
            print("Game Over detected!")
            break
            
    print("Verification Complete!")

if __name__ == "__main__":
    verify()
