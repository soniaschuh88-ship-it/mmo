import * as pulumi from "@pulumi/pulumi";
import * as azure from "@pulumi/azure-native";
import * as dockerbuild from "@pulumi/docker-build";

// ── Configuration ──────────────────────────────────────────────────────────────
const config        = new pulumi.Config();
const appPath       = config.get("appPath")      || ".";
const containerPort = config.getNumber("containerPort") || 8080;
const cpu           = config.get("cpu")          || "0.5";
const memory        = config.get("memory")       || "1Gi";

<<<<<<< HEAD
// ── LLM Backend Configuration ─────────────────────────────────────────────────
//
// bKG uses two LLM backends. Azure AI is preferred when AZURE_AI_KEY is set.
// NVIDIA NIM is the fallback.
//
// Azure AI Foundry (primary)
//   Resource:    bkg-resource  (germanywestcentral)
//   Project:     bkg
//   Subscription: 69e114eb-2f9b-4ab4-9a70-769f465bba74
//   Endpoint:    https://bkg-resource.services.ai.azure.com/
//
//   pulumi config set --secret container-gcp-typescript:azureAiKey <key>
//   Key location: Azure portal → bkg-resource → Keys and Endpoint
//
// NVIDIA NIM (fallback)
//   Obtain a free API key at: https://build.nvidia.com/
//   pulumi config set --secret container-gcp-typescript:nvidiaApiKey <key>
//
const azureAiKey        = config.requireSecret("azureAiKey");
const azureAiEndpoint   = config.get("azureAiEndpoint")   || "https://bkg-resource.services.ai.azure.com";
const azureAiDeployment = config.get("azureAiDeployment") || "gpt-4o-mini";

const nvidiaApiKey   = config.requireSecret("nvidiaApiKey");
const nvidiaBaseUrl  = config.get("nvidiaBaseUrl") || "https://integrate.api.nvidia.com/v1";
const nvidiaModel    = config.get("nvidiaModel")   || "meta/llama-3.3-70b-instruct";
=======
// NVIDIA NIM configuration — used by bifrost-wac (nvidia-nim feature).
const nvidiaApiKey  = config.getSecret("nvidiaApiKey")  ?? pulumi.output("");
const nvidiaBaseUrl = config.get("nvidiaBaseUrl") || "https://integrate.api.nvidia.com/v1";
const nvidiaModel   = config.get("nvidiaModel")   || "meta/llama-3.3-70b-instruct";
>>>>>>> 009a33b (`R3: bifrost-aigm, bifrost-server, and bifrost-wasm integration`)

const azureConfig   = new pulumi.Config("azure-native");
const location      = azureConfig.get("location") || "eastus";

// ── Resource Group ─────────────────────────────────────────────────────────────
const resourceGroup = new azure.resources.ResourceGroup("mmo-rg", {
    location,
});

// ── Azure Container Registry ───────────────────────────────────────────────────
// Equivalent to GCP Artifact Registry — stores the Docker image built from
// the Dockerfile at the workspace root.
const registry = new azure.containerregistry.Registry("mmoacr", {
    resourceGroupName: resourceGroup.name,
    location:          resourceGroup.location,
    sku:               { name: "Basic" },
    adminUserEnabled:  true,
});

// Retrieve the ACR admin credentials.
const registryCreds = azure.containerregistry.listRegistryCredentialsOutput({
    resourceGroupName: resourceGroup.name,
    registryName:      registry.name,
});
const adminUsername = registryCreds.apply(c => c.username!);
const adminPassword = registryCreds.apply(c => c.passwords![0].value!);

// ── Container Image ────────────────────────────────────────────────────────────
// Builds the image from the workspace root Dockerfile and pushes to ACR.
// The Dockerfile includes both the bifrost-server native build and the
// wasm-pack WASM bundle (bifrost/wasm → app/pkg/bifrost_wasm/).
const imageName = pulumi.concat(registry.loginServer, "/mmo-server:latest");

const image = new dockerbuild.Image("mmo-image", {
    tags: [imageName],
    context: {
        location: appPath,
    },
    // Cloud-run compatible: x86_64 only.
    platforms: ["linux/amd64"],
    push: true,
    registries: [{
        address:  registry.loginServer,
        username: adminUsername,
        password: adminPassword,
    }],
});

// ── Container Apps Environment ─────────────────────────────────────────────────
// Azure's managed serverless container runtime (equivalent to Cloud Run).
const environment = new azure.app.ManagedEnvironment("mmo-env", {
    resourceGroupName: resourceGroup.name,
    location:          resourceGroup.location,
});

// ── Container App ──────────────────────────────────────────────────────────────
// Deploys the bifrost-server + Node.js gateway container.
const containerApp = new azure.app.ContainerApp("mmo-app", {
    resourceGroupName:    resourceGroup.name,
    location:             resourceGroup.location,
    managedEnvironmentId: environment.id,
    configuration: {
        ingress: {
            external:          true,
            targetPort:        containerPort,
            allowInsecure:     false,
            transport:         "auto",
        },
        registries: [{
            server:            registry.loginServer,
            username:          adminUsername,
            passwordSecretRef: "acr-password",
        }],
        secrets: [
            {
                name:  "acr-password",
                value: adminPassword,
            },
            {
                name:  "nvidia-api-key",
                value: nvidiaApiKey,
            },
        ],
    },
    template: {
        containers: [{
            name:  "mmo-server",
            image: image.ref,
            resources: {
                cpu:    parseFloat(cpu),
                memory,
            },
            env: [
                {
<<<<<<< HEAD
                    image: image.ref,
                    resources: {
                        limits: {
                            memory,
                            cpu: cpu.toString(),
                        },
                    },
                    ports: [
                        {
                            containerPort,
                        },
                    ],
                    // LLM backend credentials for bifrost-wac (azure-ai + nvidia-nim features)
                    // and bifrost-aigm NPC dialogue (azure-ai feature).
                    // Azure AI Foundry is preferred; NVIDIA NIM is the fallback.
                    envs: [
                        // ── Azure AI Foundry (primary) ───────────────────────
                        {
                            name:  "AZURE_AI_KEY",
                            value: azureAiKey,
                        },
                        {
                            name:  "AZURE_AI_ENDPOINT",
                            value: azureAiEndpoint,
                        },
                        {
                            name:  "AZURE_AI_DEPLOYMENT",
                            value: azureAiDeployment,
                        },
                        // ── NVIDIA NIM (fallback) ────────────────────────────
                        {
                            name:  "NVIDIA_API_KEY",
                            value: nvidiaApiKey,
                        },
                        {
                            name:  "NVIDIA_NIM_BASE_URL",
                            value: nvidiaBaseUrl,
                        },
                        {
                            name:  "NVIDIA_NIM_MODEL",
                            value: nvidiaModel,
                        },
                    ],
                }
=======
                    name:  "PORT",
                    value: containerPort.toString(),
                },
                // NVIDIA NIM for bifrost-wac LLM generation (optional).
                {
                    name:        "NVIDIA_API_KEY",
                    secretRef:   "nvidia-api-key",
                },
                {
                    name:  "NVIDIA_NIM_BASE_URL",
                    value: nvidiaBaseUrl,
                },
                {
                    name:  "NVIDIA_NIM_MODEL",
                    value: nvidiaModel,
                },
>>>>>>> 009a33b (`R3: bifrost-aigm, bifrost-server, and bifrost-wasm integration`)
            ],
        }],
        scale: {
            minReplicas: 0,
            maxReplicas: 10,
        },
    },
});

// Export the public URL of the Container App.
export const url = containerApp.configuration.apply(
    c => c?.ingress?.fqdn ? `https://${c.ingress.fqdn}` : "pending"
);
export const registryLoginServer = registry.loginServer;
