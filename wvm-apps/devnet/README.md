# Local devnet

Local devnet uses kurtosis to bootstrap a local chain with all essential tooling.
It uses local images of wvm reth fork and lighthouse by default.

It has default prefounded accounts and etc.
You may use it for local development and tests.

To install kurtosis and to get a basic understanding please follow the ethereum-package docs.

## How To

clone the forked ethereum kurtosis package from https://github.com/weaveVM/ethereum-package

your docker engine should work

you should put your gcp bq-config.json file to 
```
ethereum-package/static_files/bq-config/
```
 
bq-config.json should have a key json in next format

```
{
    "dropTableBeforeSync": false,
    "projectId": "...",
    "datasetId": "...",
    "credentialsPath": "...",
    "credentialsJson": {
        "type": "...",
        "project_id": "...",
        "private_key_id": "...",
        "private_key": "...",
        ....
    }
}
```

you can run the package using 

```
kurtosis run --enclave wvmlocaldevnet $PATH_TO_LOCAL_PACKAGE_DIR $ --args-file network_params.yaml
```

stop
```
 kurtosis enclave stop wvmlocaldevnet
```
and

delete

```
 kurtosis enclave rm wvmlocaldevnet
```

