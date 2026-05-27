import * as pulumi from "@pulumi/pulumi";
import * as gcp from "@pulumi/gcp";
import * as dockerbuild from "@pulumi/docker-build";
import * as random from "@pulumi/random";

// Import the program's configuration settings.
const config = new pulumi.Config();
const imageName = config.get("imageName") || "my-app";
const appPath = config.get("appPath") || "./app";
const containerPort = config.getNumber("containerPort") || 8080;
const cpu = config.getNumber("cpu") || 1;
const memory = config.get("memory") || "1Gi";
const concurrency = config.getNumber("concurrency") || 80;

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

// Import the provider's configuration settings.
const gcpConfig = new pulumi.Config("gcp");
const location = gcpConfig.require("region");
const project = gcpConfig.require("project");

// Generate a unique Artifact Registry repository ID
const uniqueString = new random.RandomString("unique-string", {
    length: 4,
    lower: true,
    upper: false,
    numeric: true,
    special: false,
})
let repoId = uniqueString.result.apply(result => "repo-" + result);

// Create an Artifact Registry repository
const repository = new gcp.artifactregistry.Repository("repository", {
    description: "Repository for container image",
    format: "DOCKER",
    location: location,
    repositoryId: repoId,
});

// Form the repository URL
let repoUrl = pulumi.concat(location, "-docker.pkg.dev/", project, "/", repository.repositoryId);

// Create a container image for the service.
// Before running `pulumi up`, configure Docker for authentication to Artifact Registry
// as described here: https://cloud.google.com/artifact-registry/docs/docker/authentication
const image = new dockerbuild.Image("image", {
    tags: [pulumi.concat(repoUrl, "/", imageName)],
    context: {
        location: appPath,
    },
    // Cloud Run currently requires x86_64 images
    // https://cloud.google.com/run/docs/container-contract#languages
    platforms: ["linux/amd64"],
    push: true,
});

// Create a Cloud Run service definition.
const service = new gcp.cloudrun.Service("service", {
    location,
    template: {
        spec: {
            containers: [
                {
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
            ],
            containerConcurrency: concurrency,
        },
    },
});

// Create an IAM member to allow the service to be publicly accessible.
const invoker = new gcp.cloudrun.IamMember("invoker", {
    location,
    service: service.name,
    role: "roles/run.invoker",
    member: "allUsers",
});

// Export the URL of the service.
export const url = service.statuses.apply(statuses => statuses[0]?.url);
