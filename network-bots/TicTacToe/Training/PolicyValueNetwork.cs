using TorchSharp;
using static TorchSharp.torch;
using static TorchSharp.torch.nn;

namespace MiniGameNetworkBot.TicTacToe.Training;

public sealed class PolicyValueNetwork : Module<Tensor, (Tensor Policy, Tensor Value)>
{
    private readonly int _width;
    private readonly int _height;

    private readonly Module<Tensor, Tensor> _conv1;
    private readonly Module<Tensor, Tensor> _conv2;
    private readonly Module<Tensor, Tensor> _conv3;
    private readonly Module<Tensor, Tensor> _bn1;
    private readonly Module<Tensor, Tensor> _bn2;
    private readonly Module<Tensor, Tensor> _bn3;

    private readonly Module<Tensor, Tensor> _policyConv;
    private readonly Module<Tensor, Tensor> _policyBn;
    private readonly Module<Tensor, Tensor> _policyFc;

    private readonly Module<Tensor, Tensor> _valueConv;
    private readonly Module<Tensor, Tensor> _valueBn;
    private readonly Module<Tensor, Tensor> _valueFc1;
    private readonly Module<Tensor, Tensor> _valueFc2;

    public PolicyValueNetwork(int width = 15, int height = 15, int channels = 64) : base("PolicyValueNet")
    {
        _width = width;
        _height = height;

        _conv1 = Conv2d(3, channels, 3, padding: 1);
        _bn1 = BatchNorm2d(channels);
        _conv2 = Conv2d(channels, channels, 3, padding: 1);
        _bn2 = BatchNorm2d(channels);
        _conv3 = Conv2d(channels, channels, 3, padding: 1);
        _bn3 = BatchNorm2d(channels);

        _policyConv = Conv2d(channels, 2, 1);
        _policyBn = BatchNorm2d(2);
        _policyFc = Linear(2 * width * height, width * height);

        _valueConv = Conv2d(channels, 1, 1);
        _valueBn = BatchNorm2d(1);
        _valueFc1 = Linear(width * height, 64);
        _valueFc2 = Linear(64, 1);

        RegisterComponents();
    }

    public override (Tensor Policy, Tensor Value) forward(Tensor x)
    {
        x = functional.relu(_bn1.call((_conv1.call(x))));
        x = functional.relu(_bn2.call(_conv2.call(x)));
        x = functional.relu(_bn3.call(_conv3.call(x)));

        var policy = functional.relu(_policyBn.call(_policyConv.call(x)));
        policy = policy.view(-1, 2 * _width * _height);
        policy = functional.log_softmax(_policyFc.call(policy), dim: 1);

        var value = functional.relu(_valueBn.call(_valueConv.call(x)));
        value = value.view(-1, _width * _height);
        value = functional.relu(_valueFc1.call(value));
        value = functional.tanh(_valueFc2.call(value));

        return (policy, value);
    }

    public (float[] Policy, float Value) Predict(float[] boardState)
    {
        using var _ = no_grad();
        eval();

        using var input = tensor(boardState, new long[] { 1, 3, _height, _width }, device: this.parameters().First().device);
        var (policy, value) = forward(input);

        var policyArray = policy.exp().cpu().data<float>().ToArray();
        var valueFloat = value.cpu().data<float>().ToArray()[0];

        return (policyArray, valueFloat);
    }

    public void MoveToDevice(Device device) => this.to(device);

    public void SaveModel(string path) => save(path);

    public void LoadModel(string path) => load(path);
}
