import * as pulumi from "@pulumi/pulumi";
import * as azure from "@pulumi/azure-native";
import * as dockerbuild from "@pulumi/docker-build";

// ── Configuration ──────────────────────────────────────────────────────────────
const config        = new pulumi.Config();
const appPath       = config.get("appPath")      || ".";
const containerPort = config.getNumber("containerPort") || 8080;
const cpu           = config.get("cpu")          || "0.5";
const memory        = config.get("memory")       || "1Gi";

// ── LLM Backend Configuration ─────────────────────────────────────────────────
//
// bKG uses two LLM backends.  Azure AI Foundry is preferred when
// AZURE_AI_KEY is set; NVIDIA NIM is the fallback.
//
// Azure AI Foundry (primary)
//   Resource:     bkg-resource  (germanywestcentral)
//   Project:      bkg
//   Subscription: 69e114eb-2f9b-4ab4-9a70-769f465bba74
//   Endpoint:     https://bkg-resource.services.ai.azure.com/
//
//   pulumi config set --secret container-gcp-typescript:azureAiKey <key>
//   Key: Azure portal → bkg-resource → Keys and Endpoint
//
// NVIDIA NIM (fallback)
//   Obtain a free API key at: https://build.nvidia.com/
//   pulumi config set --secret container-gcp-typescript:nvidiaApiKey <key>
//
const azureAiKey        = config.requireSecret("azureAiKey");
const azureAiEndpoint   = config.get("azureAiEndpoint")   || "https://bkg-resource.services.ai.azure.com";
const azureAiDeployment = config.get("azureAiDeployment") || "gpt-4o-mini";

const nvidiaApiKey  = config.getSecret("nvidiaApiKey") ?? pulumi.output("");
const nvidiaBaseUrl = config.get("nvidiaBaseUrl") || "https://integrate.api.nvidia.com/v1";
const nvidiaModel   = config.get("nvidiaModel")   || "meta/llama-3.3-70b-instruct";

const azureConfig   = new pulumi.Config("azure-native");
const location      = azureConfig.get("location") || "eastus";

// ── Resource Group ─────────────────────────────────────────────────────────────
const resourceGroup = new azure.resources.ResourceGroup("mmo-rg", {
    location,
});

// ── Azure Container Registry ───────────────────────────────────────────────────
// Stores the Docker image built from the Dockerfile at the workspace root.
// The image includes the bifrost-server native binary and the bifrost-wasm
// WASM bundle (bifrost/wasm → app/pkg/bifrost_wasm/).
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
const imageName = pulumi.concat(registry.loginServer, "/mmo-server:latest");

const image = new dockerbuild.Image("mmo-image", {
    tags: [imageName],
    context: {
        location: appPath,
    },
    platforms: ["linux/amd64"],
    push: true,
    registries: [{
        address:  registry.loginServer,
        username: adminUsername,
        password: adminPassword,
    }],
});

// ── Container Apps Environment ─────────────────────────────────────────────────
const environment = new azure.app.ManagedEnvironment("mmo-env", {
    resourceGroupName: resourceGroup.name,
    location:          resourceGroup.location,
});

// ── Container App ──────────────────────────────────────────────────────────────
// Deploys bifrost-server + Node.js gateway.
const containerApp = new azure.app.ContainerApp("mmo-app", {
    resourceGroupName:    resourceGroup.name,
    location:             resourceGroup.location,
    managedEnvironmentId: environment.id,
    configuration: {
        ingress: {
            external:      true,
            targetPort:    containerPort,
            allowInsecure: false,
            transport:     "auto",
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
            // Azure AI Foundry — primary LLM backend for bifrost-wac + bifrost-aigm.
            {
                name:  "azure-ai-key",
                value: azureAiKey,
            },
            // NVIDIA NIM — fallback LLM backend.
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
                    name:  "PORT",
                    value: containerPort.toString(),
                },
                // ── Azure AI Foundry (primary) ─────────────────────────────
                {
                    name:      "AZURE_AI_KEY",
                    secretRef: "azure-ai-key",
                },
                {
                    name:  "AZURE_AI_ENDPOINT",
                    value: azureAiEndpoint,
                },
                {
                    name:  "AZURE_AI_DEPLOYMENT",
                    value: azureAiDeployment,
                },
                // ── NVIDIA NIM (fallback) ──────────────────────────────────
                {
                    name:      "NVIDIA_API_KEY",
                    secretRef: "nvidia-api-key",
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
