using System;
using System.Collections;
using System.Collections.Generic;
using UnityEngine;

public class ContinuousController : MonoBehaviour
{
    public static ContinuousController instance;

    protected virtual void Awake()
    {
        instance = this;
    }

    public virtual void PlaySE(AudioClip clip) { }

    // Stubs for other methods if needed
}
